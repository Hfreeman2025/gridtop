//! Optional public-API timing diagnostic; not used by CI or general tests.
use std::time::Instant;
fn main() -> anyhow::Result<()> {
    let start = Instant::now();
    let api = gridtop::api::Api::new(gridtop::api::Location::National)?;
    let init = start.elapsed().as_secs_f64() * 1000.;
    let fetch = Instant::now();
    let a = api.fetch_atlas()?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"https_client_init_ms":init,"full_forecast_fetch_and_validation_ms":fetch.elapsed().as_secs_f64()*1000.,"regions":a.regions.len()})
        )?
    );
    Ok(())
}
