// Scanline even/odd filling respects interior rings and disjoint islands.
pub fn scanline(rings: &[Vec<[f64; 2]>], y: f64) -> Vec<(f64, f64)> {
    let mut crossings = Vec::new();
    for ring in rings {
        for edge in ring.windows(2) {
            let [a, b] = [edge[0], edge[1]];
            if (a[1] > y) != (b[1] > y) {
                crossings.push(a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
            }
        }
    }
    crossings.sort_by(f64::total_cmp);
    crossings.chunks_exact(2).map(|p| (p[0], p[1])).collect()
}
