use gridtop::api;
#[test]
fn recorded_national_and_regional_responses_parse() {
    let periods = api::parse_national(include_str!("fixtures/forecast.json")).unwrap();
    assert!(periods.len() >= 90);
    assert_eq!(periods[0].intensity.forecast, 173);
    let (name, periods) = api::parse_regional(include_str!("fixtures/regional.json")).unwrap();
    assert_eq!(name, "London");
    assert_eq!(periods[0].intensity.forecast, 197);
    assert_eq!(periods[0].generationmix.len(), 9);
    let (name, _) = api::parse_regional(include_str!("fixtures/postcode.json")).unwrap();
    assert_eq!(name, "London");
}
#[test]
fn rejects_wrong_types_unknown_bands_missing_forecasts_and_api_errors() {
    for value in [
        r#"{"error":{"code":"400","message":"No match"}}"#,
        r#"{"data":[]}"#,
        r#"{"data":[{"from":"2026-11-01T00:00Z","to":"2026-11-01T00:30Z","intensity":{"forecast":null,"index":"low"}}]}"#,
        r#"{"data":[{"from":"2026-11-01T00:00Z","to":"2026-11-01T00:30Z","intensity":{"forecast":20,"index":"unknown"}}]}"#,
        r#"{"data":[{"from":"bad","to":"bad","intensity":{"forecast":"20","index":"low"}}]}"#,
    ] {
        assert!(api::parse_national(value).is_err(), "{value}");
    }
}
#[test]
fn rejects_impossible_mix_and_duplicate_fuels() {
    let valid = include_str!("fixtures/regional.json");
    assert!(api::parse_regional(&valid.replacen("\"perc\":5.3", "\"perc\":105.3", 1)).is_err());
    assert!(
        api::parse_regional(&valid.replacen("\"fuel\":\"biomass\"", "\"fuel\":\"coal\"", 1))
            .is_err()
    );
}
#[test]
fn postcode_validation_and_explicit_sw1_alias() {
    assert_eq!(api::postcode("sw1").unwrap(), "SW1A");
    assert_eq!(api::postcode("rg10").unwrap(), "RG10");
    for p in ["", "SW1A 1AA", "../intensity", "é2", "12345", "A"] {
        assert!(api::postcode(p).is_err());
    }
}

#[test]
fn truncated_forecast_is_never_labelled_as_48_hours() {
    let forecast = api::parse_national(include_str!("fixtures/forecast.json")).unwrap();
    assert!(gridtop::model::validate_forecast(&forecast).is_ok());
    assert!(gridtop::model::validate_forecast(&forecast[..95]).is_err());
}
