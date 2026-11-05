use crate::{carbon::Plan, ledger::Ledger, run_ui::safe_text};
use anyhow::Result;

pub fn print_plan(plan: &Plan, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(plan)?);
        return Ok(());
    }
    println!(
        "gridtop / carbon-aware plan\n{} · {} · data {} UTC",
        if plan.state == "recording" {
            "RECORDING · NO COMMAND WILL START"
        } else {
            "FORECAST"
        },
        plan.area,
        plan.fetched_at.format("%d %b %Y %H:%M:%S")
    );
    println!(
        "\n{} → {} UTC · expected {} min",
        plan.chosen.from.format("%a %d %b %H:%M"),
        plan.chosen.to.format("%H:%M"),
        plan.expected_seconds / 60
    );
    println!(
        "Start now  {:>7.1} g/kWh\nChosen     {:>7.1} g/kWh\nAvoided    {:+7.1} g/kWh (forecast estimate)",
        plan.immediate.average_g_per_kwh, plan.chosen.average_g_per_kwh, plan.avoided_g_per_kwh
    );
    if let Some(g) = plan.estimated_grams {
        println!(
            "Estimated  {g:+.1} g · {:.3} kWh supplied/derived",
            plan.energy_kwh.unwrap_or(0.)
        );
        if let Some(basis) = &plan.energy_assumption {
            println!("{basis}");
        }
    } else {
        println!("No energy supplied: grams are not estimated.");
    }
    let values: Vec<_> = plan.forecast.iter().map(|p| p.intensity.forecast).collect();
    let ceiling = f64::from(*values.iter().max().unwrap_or(&1));
    let line: String = crate::ui::telemetry::bars(&values, 64, 1, ceiling)
        .iter()
        .collect();
    println!(
        "\n48h forecast / column peaks\n{line}\n{} candidates · earliest minimum · half-hour-aligned starts",
        plan.candidate_count
    );
    println!("{}\n{}", plan.estimate, plan.source);
    Ok(())
}

pub fn runs(json: bool, limit: usize) -> Result<i32> {
    let ledger = Ledger::environment()?;
    let records = ledger.list(limit)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&records)?);
        return Ok(0);
    }
    println!("gridtop / local runs\n{}", ledger.directory().display());
    if records.is_empty() {
        println!("\nNo runs yet. Dry runs do not write the ledger.");
        return Ok(0);
    }
    println!("\nUTC start          last state  exit   avoided g/kWh   estimated g   program");
    for r in records {
        let intensity = r
            .avoided_g_per_kwh
            .map_or("—".into(), |v| format!("{v:+.1}"));
        let grams = r.estimated_grams.map_or("—".into(), |v| format!("{v:+.1}"));
        let status = serde_json::to_value(&r.status)?
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        println!(
            "{}  {:<10} {:>4} {:>16} {:>13}   {}",
            r.started_at.unwrap_or(r.queued_at).format("%d %b %H:%M:%S"),
            status,
            r.exit_code.map_or("—".into(), |v| v.to_string()),
            intensity,
            grams,
            safe_text(&r.program)
        );
    }
    println!(
        "\nForecast estimates versus starting immediately; constant load over --for. Not measured savings.\nGrams appear only with --kwh or --watts. Arguments, environment and child output are never stored."
    );
    Ok(0)
}
