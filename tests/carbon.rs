use chrono::{Duration, TimeZone, Utc};
use gridtop::{
    carbon::{self, Plan, Request},
    model::{Band, Intensity, Period, Reading},
    schedule,
};
fn reading() -> Reading {
    let from = Utc.with_ymd_and_hms(2026, 10, 25, 0, 0, 0).unwrap();
    let forecast: Vec<_> = (0..96)
        .map(|i| Period {
            from: from + Duration::minutes(i * 30),
            to: from + Duration::minutes((i + 1) * 30),
            intensity: Intensity {
                forecast: if (4..8).contains(&i) { 50 } else { 300 },
                actual: None,
                index: Band::Moderate,
            },
            generationmix: vec![],
        })
        .collect();
    Reading {
        area: "London".into(),
        fetched_at: from,
        current: forecast[0].clone(),
        mix_from: from,
        mix_to: from + Duration::minutes(30),
        generation_mix: vec![],
        forecast,
        sample: false,
    }
}
fn request() -> Request {
    Request {
        length: Duration::hours(1),
        within: Some(Duration::hours(12)),
        ..Default::default()
    }
}
#[test]
fn compares_against_immediate_partial_slots_and_keeps_signed_deltas() {
    let mut r = reading();
    let now = r.forecast[0].from + Duration::minutes(14);
    r.fetched_at = now;
    let p = Plan::new(&r, now, &request()).unwrap();
    assert_eq!(p.chosen.from, r.forecast[4].from);
    assert_eq!(p.immediate.from, now);
    assert_eq!(p.immediate.average_g_per_kwh, 300.);
    assert_eq!(p.avoided_g_per_kwh, 250.);
    assert_eq!(p.estimated_grams, None);
    r.forecast[0].intensity.forecast = 0;
    let req = Request {
        within: Some(Duration::hours(1)),
        length: Duration::minutes(30),
        ..Default::default()
    };
    let p = Plan::new(&r, now, &req).unwrap();
    assert!(
        p.avoided_g_per_kwh < 0.,
        "alignment can make delaying worse; do not clamp it"
    );
}
#[test]
fn integration_weights_fractional_edges_and_rejects_missing_coverage() {
    let r = reading();
    let t = r.forecast[0].from;
    let avg = schedule::average(
        &r.forecast,
        t + Duration::minutes(105),
        t + Duration::minutes(135),
    )
    .unwrap();
    assert_eq!(avg, 175.);
    assert!(
        schedule::average(
            &r.forecast,
            t - Duration::minutes(1),
            t + Duration::minutes(1)
        )
        .is_err()
    );
}

#[test]
fn comparison_preserves_sub_millisecond_planning_instants() {
    let r = reading();
    let start = r.forecast[0].from;
    let shift = Duration::nanoseconds(123_456_789);
    let from = start + Duration::minutes(105) + shift;
    let to = start + Duration::minutes(135) + shift;
    let expected = (300. * (900. - 0.123456789) + 50. * (900. + 0.123456789)) / 1800.;
    let actual = schedule::average(&r.forecast, from, to).unwrap();
    assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
}
#[test]
fn deadline_caps_full_runtime_and_dst_offsets_are_unambiguous() {
    let r = reading();
    let now = r.forecast[0].from;
    let a = carbon::deadline("2026-10-25T01:30:00+01:00").unwrap();
    let b = carbon::deadline("2026-10-25T01:30:00+00:00").unwrap();
    assert_eq!((b - a).num_hours(), 1);
    for bad in ["2026-10-25T01:30:00", "2026-03-29T01:30:00", "18:00"] {
        assert!(carbon::deadline(bad).is_err());
    }
    let p = Plan::new(
        &r,
        now,
        &Request {
            deadline: Some(b),
            length: Duration::hours(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(p.chosen.to <= b);
    assert!(
        Plan::new(
            &r,
            now,
            &Request {
                deadline: Some(now),
                within: None,
                ..request()
            }
        )
        .is_err()
    );
    assert!(
        Plan::new(
            &r,
            now,
            &Request {
                deadline: Some(now + Duration::minutes(45)),
                within: None,
                ..request()
            }
        )
        .is_err()
    );
}
#[test]
fn grams_require_explicit_energy_and_power_uses_expected_duration() {
    let r = reading();
    let t = r.forecast[0].from;
    let a = Plan::new(
        &r,
        t,
        &Request {
            kwh: Some(0.4),
            ..request()
        },
    )
    .unwrap();
    let b = Plan::new(
        &r,
        t,
        &Request {
            watts: Some(400.),
            ..request()
        },
    )
    .unwrap();
    assert_eq!(a.estimated_grams, Some(100.));
    assert_eq!(a.estimated_grams, b.estimated_grams);
    for bad in [f64::NAN, f64::INFINITY, -1., 0.] {
        assert!(
            Plan::new(
                &r,
                t,
                &Request {
                    kwh: Some(bad),
                    ..request()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn final_gate_rejects_recordings_stale_missed_and_deadline_overruns() {
    let mut r = reading();
    let t = r.forecast[0].from;
    let p = Plan::new(&r, t, &request()).unwrap();
    let at = p.chosen.from;
    r.current = r.forecast[4].clone();
    r.fetched_at = at;
    assert_eq!(p.launch_average(&r, at).unwrap(), 50.);
    assert!(p.launch_average(&r, at - Duration::seconds(1)).is_err());
    assert!(p.launch_average(&r, at + Duration::minutes(1)).is_err());
    r.sample = true;
    assert!(p.launch_average(&r, at).is_err());
    r.sample = false;
    r.fetched_at = t;
    assert!(p.launch_average(&r, at).is_err());
    r.fetched_at = at;
    let mut p = p;
    p.horizon_end = at + Duration::hours(1);
    assert!(p.launch_average(&r, at + Duration::seconds(1)).is_err());
}

#[test]
fn refreshed_screen_preserves_start_and_original_counterfactual() {
    let mut r = reading();
    let now = r.forecast[0].from;
    let mut p = Plan::new(&r, now, &request()).unwrap();
    let selected = p.chosen.from;
    for slot in &mut r.forecast[4..8] {
        slot.intensity.forecast = 80;
    }
    r.fetched_at = now + Duration::minutes(1);
    p.refresh_display(&r).unwrap();
    assert_eq!(p.chosen.from, selected);
    assert_eq!(p.immediate.average_g_per_kwh, 300.);
    assert_eq!(p.chosen.average_g_per_kwh, 80.);
    assert_eq!(p.avoided_g_per_kwh, 220.);
    assert_eq!(p.fetched_at, r.fetched_at);
    r.area = "another region".into();
    assert!(p.refresh_display(&r).is_err());
}

#[test]
fn fallback_is_not_available_for_recordings_or_planning_errors() {
    use gridtop::carbon_cli::{Acquisition, resolve_fetch};
    assert!(resolve_fetch(Err(anyhow::anyhow!("offline fixture")), false).is_err());
    assert!(matches!(
        resolve_fetch(Err(anyhow::anyhow!("offline fixture")), true).unwrap(),
        Acquisition::Immediate(_)
    ));
    let mut r = reading();
    r.sample = true;
    assert!(resolve_fetch(Ok(r), true).is_err());
}
