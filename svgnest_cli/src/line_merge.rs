use std::collections::HashMap;

use crate::svg_parser::{Polygon, Point};

const MERGE_TOLERANCE: f64 = 1e-6;

fn key_for_point(p: &Point) -> (i64, i64) {
    ((p.x / MERGE_TOLERANCE).round() as i64, (p.y / MERGE_TOLERANCE).round() as i64)
}

/// Merge duplicate line segments across all polygons.
/// Each edge is stored as an unordered pair of points so orientation does not matter.
pub fn merge_lines(polys: &[Polygon]) -> Vec<Polygon> {
    let mut edges: HashMap<((i64, i64), (i64, i64)), (Point, Point)> = HashMap::new();

    for poly in polys {
        if poly.points.len() < 2 {
            continue;
        }
        let mut segments: Vec<(Point, Point)> = poly.points.windows(2).map(|w| (w[0], w[1])).collect();
        if poly.closed && poly.points.len() > 2 {
            let last = poly.points.len() - 1;
            segments.push((poly.points[last], poly.points[0]));
        }
        for (a, b) in segments {
            let ka = key_for_point(&a);
            let kb = key_for_point(&b);
            let key = if ka <= kb { (ka, kb) } else { (kb, ka) };
            edges.entry(key).or_insert((a, b));
        }
    }

    let mut result: Vec<Polygon> = edges
        .into_iter()
        .map(|(_, (a, b))| Polygon { id: 0, points: vec![a, b], closed: false })
        .collect();
    result.sort_by(|a, b| {
        a.points[0]
            .x
            .partial_cmp(&b.points[0].x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, p) in result.iter_mut().enumerate() {
        p.id = i;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_segments() {
        let p1 = Polygon { id: 0, points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 0.0 }], closed: false };
        let p2 = Polygon { id: 1, points: vec![Point { x: 1.0, y: 0.0 }, Point { x: 0.0, y: 0.0 }], closed: false };
        let p3 = Polygon { id: 2, points: vec![Point { x: 2.0, y: 2.0 }, Point { x: 3.0, y: 2.0 }], closed: false };
        let merged = merge_lines(&[p1, p2, p3]);
        assert_eq!(merged.len(), 2);
    }
}

