use std::collections::HashMap;

use crate::svg_parser::Point;
use crate::geometry::{minkowski_difference, offset_polygon};

pub struct NfpCache {
    cache: HashMap<(usize, usize, i64, i64), Vec<Point>>, // key with quantized angles
}

impl NfpCache {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    pub fn get_or_generate(
        &mut self,
        a_id: usize,
        b_id: usize,
        a_angle: f64,
        b_angle: f64,
        a: &[Point],
        b: &[Point],
    ) -> Vec<Point> {
        let key = (
            a_id,
            b_id,
            (a_angle * 1000.0) as i64,
            (b_angle * 1000.0) as i64,
        );
        if let Some(v) = self.cache.get(&key) {
            return v.clone();
        }
        let nfp = minkowski_difference(a, b);
        self.cache.insert(key, nfp.clone());
        nfp
    }
}

/// Simple outer no-fit polygon using Minkowski difference.
pub fn no_fit_polygon(a: &[Point], b: &[Point]) -> Vec<Point> {
    minkowski_difference(a, b)
}

/// Generate inner fit polygons by offsetting the container and computing the
/// outer no-fit polygon for each offset polygon.
pub fn inner_fit_polygon(container: &[Point], part: &[Point], spacing: f64) -> Vec<Vec<Point>> {
    let offsets = offset_polygon(container, -spacing.abs());
    offsets
        .into_iter()
        .map(|poly| minkowski_difference(&poly, part))
        .collect()
}
