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

use geo::{prelude::*, LineString as GeoLineString, MultiPolygon, Polygon as GeoPolygon};
use geo_clipper::{Clipper, EndType, JoinType};

fn to_geo_polygon(points: &[Point]) -> GeoPolygon<f64> {
    let exterior: GeoLineString<f64> = points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>().into();
    GeoPolygon::new(exterior, vec![])
}

fn to_geo_polygon_translated(points: &[Point], tx: f64, ty: f64) -> GeoPolygon<f64> {
    let exterior: GeoLineString<f64> = points
        .iter()
        .map(|p| (p.x + tx, p.y + ty))
        .collect::<Vec<_>>()
        .into();
    GeoPolygon::new(exterior, vec![])
}

/// Offset a polygon by the given delta using the Clipper library.
pub fn offset_polygon(points: &[Point], delta: f64) -> Vec<Vec<Point>> {
    if points.is_empty() {
        return Vec::new();
    }
    let poly = to_geo_polygon(points);
    let mp = poly.offset(delta, JoinType::Miter(1.0), EndType::ClosedPolygon, CLIPPER_SCALE);
    mp.0
        .into_iter()
        .map(|p| {
            p.exterior()
                .points()
                .map(|c| Point { x: c.x(), y: c.y() })
                .collect()
        })
        .collect()
}

/// General Minkowski difference using the Clipper library.
///
/// This implementation mirrors the JavaScript version used by SVGnest and
/// correctly handles concave polygons by constructing the Minkowski sum of `a`
/// with the negated `b` polygon and unioning the intermediate quads via
/// `geo_clipper::Clipper`.
pub fn minkowski_difference_clip(a: &[Point], b: &[Point]) -> Vec<Point> {
    use std::cmp::Ordering;

    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }

    let la = a.len();
    let lb = b.len();

    // Precompute (-B) + A point matrices (Minkowski sum of A with inverted B)
    let mut sum: Vec<Vec<Point>> = Vec::with_capacity(lb);
    for pb in b {
        let row: Vec<Point> = a
            .iter()
            .map(|pa| Point {
                x: pa.x - pb.x,
                y: pa.y - pb.y,
            })
            .collect();
        sum.push(row);
    }

    // Build quads from the point matrices
    let mut quads: Vec<Vec<Point>> = Vec::new();
    for i in 0..lb { // path is closed
        for j in 0..la {
            let mut poly = vec![
                sum[i % lb][j % la],
                sum[(i + 1) % lb][j % la],
                sum[(i + 1) % lb][(j + 1) % la],
                sum[i % lb][(j + 1) % la],
            ];
            if polygon_area(&poly) < 0.0 {
                poly.reverse();
            }
            quads.push(poly);
        }
    }

    // Union all quads using Clipper
    let mut acc: Option<MultiPolygon<f64>> = None;
    for quad in &quads {
        let g = to_geo_polygon(quad);
        acc = Some(match acc {
            Some(mp) => Clipper::union(&mp, &g, CLIPPER_SCALE),
            None => MultiPolygon(vec![g]),
        });
    }

    let mp = match acc {
        Some(mp) => mp,
        None => return Vec::new(),
    };

    // Select the polygon with the smallest (most negative) area
    let poly_opt = mp.0.into_iter().min_by(|p1, p2| {
        p1.signed_area()
            .partial_cmp(&p2.signed_area())
            .unwrap_or(Ordering::Equal)
    });

    if let Some(poly) = poly_opt {
        let mut pts: Vec<Point> = poly
            .exterior()
            .points()
            .map(|c| Point { x: c.x(), y: c.y() })
            .collect();
        // Translate by the first vertex of B
        for p in &mut pts {
            p.x += b[0].x;
            p.y += b[0].y;
        }
        pts
    } else {
        Vec::new()
    }
}

/// Returns true if the two polygons intersect when translated by (ax,ay) and (bx,by)
pub fn polygons_intersect(a: &[Point], b: &[Point], ax: f64, ay: f64, bx: f64, by: f64) -> bool {
    let pa = to_geo_polygon_translated(a, ax, ay);
    let pb = to_geo_polygon_translated(b, bx, by);
    !Clipper::intersection(&pa, &pb, CLIPPER_SCALE).0.is_empty()
}

/// Returns true if polygon `b` translated by (bx,by) lies completely inside
/// polygon `a` translated by (ax,ay).
pub fn polygon_contains_polygon(a: &[Point], b: &[Point], ax: f64, ay: f64, bx: f64, by: f64) -> bool {
    for p in b {
        if !point_in_polygon(a, p.x + bx - ax, p.y + by - ay) {
            return false;
        }
    }
    true
}

/// Returns true if point (x,y) lies inside the polygon using even-odd rule.
pub fn point_in_polygon(poly: &[Point], x: f64, y: f64) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let xi = poly[i].x;
        let yi = poly[i].y;
        let xj = poly[j].x;
        let yj = poly[j].y;
        let intersect = ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi + 1e-9) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
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
