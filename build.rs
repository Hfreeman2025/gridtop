#[path = "build_support/raster.rs"]
mod raster;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};
#[derive(Deserialize)]
struct Collection {
    features: Vec<Feature>,
}
#[derive(Deserialize)]
struct Feature {
    properties: Properties,
    geometry: Geometry,
}
#[derive(Deserialize)]
struct Properties {
    #[serde(rename = "ID")]
    id: u8,
}
#[derive(Deserialize)]
struct Geometry {
    coordinates: Vec<Vec<Vec<[f64; 2]>>>,
}
fn main() {
    println!("cargo:rerun-if-changed=data/regions.geojson");
    println!("cargo:rerun-if-changed=build_support/raster.rs");
    let raw = fs::read("data/regions.geojson").expect("boundary data");
    let data: Collection = serde_json::from_slice(&raw).expect("valid boundary GeoJSON");
    assert_eq!(data.features.len(), 14);
    let mut bounds = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for p in data
        .features
        .iter()
        .flat_map(|f| &f.geometry.coordinates)
        .flatten()
        .flatten()
    {
        assert!(p[0].is_finite() && p[1].is_finite());
        bounds[0] = bounds[0].min(p[0]);
        bounds[1] = bounds[1].min(p[1]);
        bounds[2] = bounds[2].max(p[0]);
        bounds[3] = bounds[3].max(p[1]);
    }
    let (w, h) = (384usize, 768usize);
    let dx = (bounds[2] - bounds[0]) / w as f64;
    let dy = (bounds[3] - bounds[1]) / h as f64;
    let mut pixels = vec![0u8; w * h];
    for feature in &data.features {
        // Official DNO IDs differ from Carbon Intensity region IDs. Explicit join.
        let id = match feature.properties.id {
            17 => 1,
            18 => 2,
            16 => 3,
            15 => 4,
            23 => 5,
            13 => 6,
            21 => 7,
            14 => 8,
            11 => 9,
            10 => 10,
            22 => 11,
            20 => 12,
            12 => 13,
            19 => 14,
            other => panic!("unknown DNO boundary ID {other}"),
        };
        for y in 0..h {
            let world_y = bounds[3] - (y as f64 + 0.5) * dy;
            for polygon in &feature.geometry.coordinates {
                for (a, b) in raster::scanline(polygon, world_y) {
                    let left = ((a - bounds[0]) / dx - 0.5).ceil().max(0.0) as usize;
                    let right = ((b - bounds[0]) / dx - 0.5).ceil().max(0.0) as usize;
                    for x in left..right.min(w) {
                        pixels[y * w + x] = id;
                    }
                }
            }
        }
    }
    for id in 1..=14 {
        assert!(pixels.contains(&id), "region missing from raster: {id}");
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("regions.bin"), &pixels).expect("write raster");
    fs::write(
        out.join("bounds.rs"),
        format!(
            "pub const ASPECT:f64={}; pub const BOUNDS:[f64;4]={bounds:?};",
            (bounds[2] - bounds[0]) / (bounds[3] - bounds[1])
        ),
    )
    .expect("bounds");
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 64"><title>Great Britain</title><desc>Supported by National Energy SO Open Data</desc><g fill="#9bcb76">"##,
    );
    for y in 0..64 {
        for x in 0..32 {
            if pixels[y * 12 * w + x * 12] > 0 {
                svg.push_str(&format!(r#"<path d="M{x} {y}h1v1h-1z"/>"#));
            }
        }
    }
    svg.push_str("</g></svg>");
    fs::write(out.join("favicon.svg"), svg).expect("favicon");
}
