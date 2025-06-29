use crate::svg_parser::{Point, Polygon};
use geo::{Area, BoundingRect, LineString, Rotate, point};

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

/// Rotate a collection of polygons by the given angle.
pub fn rotate_polygons(polys: &[Polygon], angle_deg: f64) -> Vec<Polygon> {
    polys
        .iter()
        .map(|p| Polygon {
            id: p.id,
            points: rotate_polygon(&p.points, angle_deg),
            closed: p.closed,
        })
        .collect()
}

/// Translate polygons so the minimum x and y coordinates become the origin
pub fn normalize_polygons(polys: &mut [Polygon]) {
    if polys.is_empty() {
        return;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for poly in polys.iter() {
        for p in &poly.points {
            if p.x < min_x {
                min_x = p.x;
            }
            if p.y < min_y {
                min_y = p.y;
            }
        }
    }
    if min_x == 0.0 && min_y == 0.0 {
        return;
    }
    for poly in polys.iter_mut() {
        for p in &mut poly.points {
            p.x -= min_x;
            p.y -= min_y;
        }
    }
}

/// Bounding box that encompasses all provided polygons.
pub fn get_polygons_bounds(polys: &[Polygon]) -> Option<Bounds> {
    let mut iter = polys.iter().filter_map(|p| get_polygon_bounds(&p.points));
    let first = iter.next()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x + first.width;
    let mut max_y = first.y + first.height;
    for b in iter {
        min_x = min_x.min(b.x);
        min_y = min_y.min(b.y);
        max_x = max_x.max(b.x + b.width);
        max_y = max_y.max(b.y + b.height);
    }
    Some(Bounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    })
}

use geo_clipper::{Clipper, EndType, JoinType};
use geo::{LineString as GeoLineString, Polygon as GeoPolygon};

/// Offset a polygon by the given delta using geo-clipper.
pub fn offset_polygon(points: &[Point], delta: f64) -> Vec<Vec<Point>> {
    if points.len() < 3 {
        return Vec::new();
    }
    let coords: Vec<_> = points.iter().map(|p| (p.x, p.y)).collect();
    let poly = GeoPolygon::new(GeoLineString::from(coords), vec![]);
    let result = poly.offset(
        delta,
        JoinType::Miter(2.0),
        EndType::ClosedPolygon,
        CLIPPER_SCALE,
    );
    result
        .into_iter()
        .map(|p| {
            p.exterior()
                .points_iter()
                .map(|c| Point { x: c.x(), y: c.y() })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Naive Minkowski difference of two polygons.
/// This implementation approximates the result using bounding boxes and is not
/// equivalent to the full algorithm.
pub fn minkowski_difference(a: &[Point], b: &[Point]) -> Vec<Vec<Point>> {
    let ab = match get_polygon_bounds(a) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let bb = match get_polygon_bounds(b) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let dx = ab.x - bb.x;
    let dy = ab.y - bb.y;
    let pts = vec![
        Point { x: dx, y: dy },
        Point {
            x: dx + ab.width - bb.width,
            y: dy,
        },
        Point {
            x: dx + ab.width - bb.width,
            y: dy + ab.height - bb.height,
        },
        Point {
            x: dx,
            y: dy + ab.height - bb.height,
        },
    ];
    vec![pts]
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
