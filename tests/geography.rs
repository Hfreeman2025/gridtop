#[path = "../build_support/raster.rs"]
mod raster;
#[test]
fn scanline_respects_holes_and_islands() {
    let box_ring = |a: f64, b: f64, c: f64, d: f64| vec![[a, b], [c, b], [c, d], [a, d], [a, b]];
    let rings = vec![
        box_ring(0., 0., 10., 10.),
        box_ring(3., 3., 7., 7.),
        box_ring(20., 0., 22., 2.),
    ];
    assert_eq!(raster::scanline(&rings, 5.), vec![(0., 3.), (7., 10.)]);
    assert_eq!(raster::scanline(&rings, 1.), vec![(0., 10.), (20., 22.)]);
    assert!(raster::scanline(&rings, 11.).is_empty());
}
#[test]
fn public_atlas_requires_all_regions_and_continuity() {
    use gridtop::atlas::Atlas;
    let text = include_str!("fixtures/atlas.json");
    let a = Atlas::demo().unwrap();
    assert_eq!(a.regions.len(), 14);
    assert_eq!(a.regions[12].id, 13);
    assert_eq!(a.regions[12].name, "London");
    assert_eq!(a.regions[0].periods.len(), 96);
    assert!(!a.fresh(chrono::Utc::now()));
    let mut v: serde_json::Value = serde_json::from_str(text).unwrap();
    v["data"][0]["regions"].as_array_mut().unwrap().remove(0);
    assert!(Atlas::parse(&v.to_string(), chrono::Utc::now(), false).is_err());
    let mut v: serde_json::Value = serde_json::from_str(text).unwrap();
    v["data"][1]["from"] = serde_json::json!("2026-11-01T00:00Z");
    assert!(Atlas::parse(&v.to_string(), chrono::Utc::now(), false).is_err());
}
