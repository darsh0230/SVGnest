use crate::svg_parser::Point;
use geo::{Area, BoundingRect, Rotate, LineString, point};

/// Bounding box of a polygon
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Default scale factor used when interfacing with Clipper
pub const CLIPPER_SCALE: f64 = 10_000_000.0;

/// Default curve tolerance when approximating curves
pub const CURVE_TOLERANCE: f64 = 0.3;

fn to_linestring(points: &[Point]) -> LineString<f64> {
    points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>().into()
}

/// Calculate the rectangular bounds of the polygon.
/// Returns `None` if there are fewer than 3 points.
pub fn get_polygon_bounds(points: &[Point]) -> Option<Bounds> {
    if points.len() < 3 {
        return None;
    }
    let ls = to_linestring(points);
    let rect = ls.bounding_rect()?;
    Some(Bounds {
        x: rect.min().x,
        y: rect.min().y,
        width: rect.width(),
        height: rect.height(),
    })
}

/// Signed area of the polygon. A negative value indicates
/// counter-clockwise winding, matching the JavaScript implementation.
pub fn polygon_area(points: &[Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        area += (points[j].x + points[i].x) * (points[j].y - points[i].y);
        j = i;
    }
    0.5 * area
}

/// Rotate polygon by the given angle in degrees around the origin.
pub fn rotate_polygon(points: &[Point], angle_deg: f64) -> Vec<Point> {
    if points.is_empty() {
        return Vec::new();
    }
    let ls = to_linestring(points);
    let origin = point!(x: 0.0, y: 0.0);
    let rotated = ls.rotate_around_point(angle_deg, origin);
    rotated
        .points()
        .map(|c| Point { x: c.x(), y: c.y() })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_of_square() {
        let pts = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ];
        assert_eq!(polygon_area(&pts), -1.0);
        let bounds = get_polygon_bounds(&pts).unwrap();
        assert_eq!(bounds.width, 1.0);
        assert_eq!(bounds.height, 1.0);
    }

    #[test]
    fn area_of_triangle_ccw() {
        let pts = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.0, y: 1.0 },
        ];
        assert!((polygon_area(&pts) + 0.5).abs() < 1e-6);
    }

    #[test]
    fn rotate_preserves_bounds() {
        let pts = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ];
        let rotated = rotate_polygon(&pts, 90.0);
        let b = get_polygon_bounds(&rotated).unwrap();
        assert!((b.width - 1.0).abs() < 1e-6);
        assert!((b.height - 1.0).abs() < 1e-6);
    }

    #[test]
    fn degenerate_polygon() {
        let pts = vec![Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 0.0 }];
        assert_eq!(polygon_area(&pts), 0.0);
        assert!(get_polygon_bounds(&pts).is_none());
    }
}
