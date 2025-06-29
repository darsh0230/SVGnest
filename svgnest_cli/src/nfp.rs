use crate::geometry::{minkowski_difference, offset_polygon};
use crate::svg_parser::Point;

/// Outer no-fit polygon of two shapes.
/// This is a very approximate implementation intended for testing.
pub fn no_fit_polygon(a: &[Point], b: &[Point], spacing: f64) -> Option<Vec<Point>> {
    let mut nfp = minkowski_difference(a, b);
    if nfp.is_empty() {
        return None;
    }
    if spacing != 0.0 {
        nfp = nfp
            .into_iter()
            .flat_map(|poly| offset_polygon(&poly, spacing))
            .collect();
    }
    nfp.into_iter().next()
}

/// Inner fit polygon of a part inside a bin.
pub fn inner_fit_polygon(bin: &[Point], part: &[Point], spacing: f64) -> Option<Vec<Point>> {
    let mut nfp = minkowski_difference(bin, part);
    if nfp.is_empty() {
        return None;
    }
    if spacing != 0.0 {
        nfp = nfp
            .into_iter()
            .flat_map(|poly| offset_polygon(&poly, -spacing))
            .collect();
    }
    nfp.into_iter().next()
}
