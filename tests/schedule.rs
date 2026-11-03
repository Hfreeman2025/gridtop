use chrono::{DateTime, Duration, Utc};
use gridtop::{
    model::{Band, Intensity, Period},
    schedule::{self, Clock, Decision},
};

fn time(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}
fn base() -> DateTime<Utc> {
    time("2026-11-01T00:00:00Z")
}
fn series(values: &[u16]) -> Vec<Period> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| Period {
            from: base() + Duration::minutes(i as i64 * 30),
            to: base() + Duration::minutes((i as i64 + 1) * 30),
            intensity: Intensity {
                forecast: *v,
                actual: None,
                index: Band::Low,
            },
            generationmix: vec![],
        })
        .collect()
}
#[test]
fn chooses_contiguous_minimum_and_earliest_tie() {
    let s = series(&[200, 40, 20, 200, 40, 20]);
    let w = schedule::best_window(&s, base(), Duration::hours(3), Duration::hours(1)).unwrap();
    assert_eq!(w.from, base() + Duration::minutes(30));
    assert_eq!(w.average_g_per_kwh, 30.0);
}
#[test]
fn never_includes_past_and_weights_partial_intervals() {
    let s = series(&[200, 200, 20, 20]);
    let w = schedule::best_window(
        &s,
        base() + Duration::minutes(15),
        Duration::minutes(60),
        Duration::minutes(45),
    )
    .unwrap();
    assert_eq!(w.from, base() + Duration::minutes(30));
    assert_eq!(w.to, base() + Duration::minutes(75));
    assert_eq!(w.average_g_per_kwh, 140.0);
}
#[test]
fn earliest_aligned_start_wins_a_tie_for_a_non_slot_duration() {
    let w = schedule::best_window(
        &series(&[200, 10, 10, 200]),
        base(),
        Duration::hours(2),
        Duration::minutes(45),
    )
    .unwrap();
    assert_eq!(w.from, base() + Duration::minutes(30));
    assert_eq!(w.average_g_per_kwh, 10.0);
}
#[test]
fn duration_equal_to_horizon_and_coverage_boundary() {
    let s = series(&[20, 40]);
    let w = schedule::best_window(&s, base(), Duration::hours(1), Duration::hours(1)).unwrap();
    assert_eq!(w.to, base() + Duration::hours(1));
    assert_eq!(w.average_g_per_kwh, 30.0);
    assert!(
        schedule::best_window(
            &s,
            base() + Duration::minutes(1),
            Duration::hours(1),
            Duration::hours(1)
        )
        .is_err()
    );
}

#[test]
fn mid_slot_now_skips_the_elapsed_slot_even_when_it_is_cleanest() {
    let now =
        base() + Duration::minutes(14) + Duration::seconds(17) + Duration::microseconds(123_456);
    let w = schedule::best_window(
        &series(&[10, 40, 40, 200, 200, 200]),
        now,
        Duration::hours(2),
        Duration::hours(1),
    )
    .unwrap();
    assert_eq!(w.from, base() + Duration::minutes(30));
    assert_eq!(w.to, base() + Duration::minutes(90));
    assert_eq!(w.average_g_per_kwh, 40.0);
}

#[test]
fn mid_slot_horizon_never_creates_an_unaligned_tail_candidate() {
    let now =
        base() + Duration::minutes(14) + Duration::seconds(17) + Duration::microseconds(123_456);
    let w = schedule::best_window(
        &series(&[250, 220, 180, 120, 60, 20]),
        now,
        Duration::hours(2),
        Duration::hours(1),
    )
    .unwrap();
    assert_eq!(w.from, base() + Duration::hours(1));
    assert_eq!(w.to, base() + Duration::hours(2));
    assert_eq!(w.average_g_per_kwh, 150.0);
    assert!(w.to <= now + Duration::hours(2));
}

#[test]
fn mid_slot_cleanest_wait_holds_until_a_complete_api_slot() {
    let now = base() + Duration::minutes(14) + Duration::seconds(17);
    let w = schedule::best_window(
        &series(&[10, 40, 40, 200]),
        now,
        Duration::hours(1),
        Duration::minutes(30),
    )
    .unwrap();
    assert_eq!(w.from, base() + Duration::minutes(30));
    assert_eq!(w.to, base() + Duration::hours(1));
    assert_eq!(
        schedule::at(&FakeClock(now), w.from, w.to).unwrap(),
        Decision::Hold
    );
    assert_eq!(
        schedule::at(&FakeClock(w.from), w.from, w.to).unwrap(),
        Decision::Run
    );
}

#[test]
fn mid_slot_horizon_must_fit_a_complete_aligned_window() {
    assert!(
        schedule::best_window(
            &series(&[10, 10, 10]),
            base() + Duration::minutes(14),
            Duration::hours(1),
            Duration::hours(1),
        )
        .is_err()
    );
}

#[test]
fn non_slot_duration_still_starts_at_an_api_boundary() {
    let w = schedule::best_window(
        &series(&[10, 20, 100, 200]),
        base() + Duration::minutes(14),
        Duration::minutes(90),
        Duration::minutes(45),
    )
    .unwrap();
    assert_eq!(w.from, base() + Duration::minutes(30));
    assert_eq!(w.to, base() + Duration::minutes(75));
    assert!((w.average_g_per_kwh - 140.0 / 3.0).abs() < 1e-9);
}
#[test]
fn rejects_gaps_overlaps_empty_and_invalid_windows() {
    let mut s = series(&[20, 40, 10]);
    s[1].from += Duration::minutes(1);
    assert!(schedule::best_window(&s, base(), Duration::hours(1), Duration::minutes(30)).is_err());
    assert!(schedule::best_window(&[], base(), Duration::hours(1), Duration::minutes(30)).is_err());
    assert!(
        schedule::best_window(
            &series(&[20]),
            base(),
            Duration::minutes(10),
            Duration::minutes(30)
        )
        .is_err()
    );
    assert!(
        schedule::best_window(
            &series(&[20]),
            base(),
            Duration::minutes(30),
            Duration::zero()
        )
        .is_err()
    );
}
struct FakeClock(DateTime<Utc>);
impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}
#[test]
fn threshold_is_strict_and_requires_current_fresh_data() {
    let p = series(&[50]).remove(0);
    assert_eq!(
        schedule::below(&FakeClock(base()), &p, 51, base()),
        Decision::Run
    );
    assert_eq!(
        schedule::below(&FakeClock(base()), &p, 50, base()),
        Decision::Hold
    );
    assert_eq!(
        schedule::below(&FakeClock(base() + Duration::minutes(30)), &p, 51, base()),
        Decision::Hold
    );
    assert_eq!(
        schedule::below(&FakeClock(base() + Duration::minutes(5)), &p, 51, base()),
        Decision::Hold
    );
    assert_eq!(
        schedule::below(&FakeClock(base() - Duration::seconds(1)), &p, 51, base()),
        Decision::Hold
    );
}
#[test]
fn injected_clock_obeys_launch_and_expiry_boundaries() {
    let target = base() + Duration::hours(2);
    let expires = target + Duration::minutes(30);
    assert_eq!(
        schedule::at(&FakeClock(target - Duration::seconds(1)), target, expires).unwrap(),
        Decision::Hold
    );
    assert_eq!(
        schedule::at(&FakeClock(target), target, expires).unwrap(),
        Decision::Run
    );
    assert!(schedule::at(&FakeClock(expires), target, expires).is_err());
}
#[test]
fn duration_parser_rejects_untrusted_text_without_panicking() {
    for invalid in [
        "",
        "0h",
        "49h",
        "-2h",
        "1.5h",
        "2d",
        "9999999999999999h",
        "é",
        "2💡",
    ] {
        assert!(schedule::duration(invalid).is_err(), "{invalid}");
    }
    assert_eq!(schedule::duration("90m").unwrap(), Duration::minutes(90));
}

#[test]
fn matches_an_independent_slot_aligned_minute_resolution_oracle() {
    for seed in 0..20u16 {
        let values: Vec<_> = (0..8u16).map(|i| (i * 73 + seed * 37) % 250).collect();
        let s = series(&values);
        for offset in [0, 5, 15] {
            for length in [15, 30, 45, 60, 75] {
                let now = base() + Duration::minutes(offset);
                let answer = schedule::best_window(
                    &s,
                    now,
                    Duration::minutes(180),
                    Duration::minutes(length),
                )
                .unwrap();
                let mut expected = None;
                for start in offset..=offset + 180 - length {
                    if start % 30 != 0 {
                        continue;
                    }
                    let sum: u64 = (start..start + length)
                        .map(|minute| u64::from(values[(minute / 30) as usize]))
                        .sum();
                    let avg = sum as f64 / length as f64;
                    if expected.as_ref().is_none_or(|(_, best)| avg < *best - 1e-9) {
                        expected = Some((start, avg));
                    }
                }
                let (start, average) = expected.unwrap();
                assert_eq!(answer.from, base() + Duration::minutes(start));
                assert!((answer.average_g_per_kwh - average).abs() < 1e-9);
            }
        }
    }
}
