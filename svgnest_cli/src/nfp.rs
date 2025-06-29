use std::collections::HashMap;

use crate::svg_parser::Point;
use crate::geometry::{minkowski_difference_clip, offset_polygon, get_polygon_bounds, CLIPPER_SCALE};
use geo::{LineString, Polygon as GeoPolygon, Translate};
use geo_clipper::Clipper;

pub struct NfpCache {
    cache: HashMap<(usize, usize, i64, i64), Vec<Point>>, // key with quantized angles
    pub angle_precision: f64,
}

impl NfpCache {
    pub const DEFAULT_ANGLE_PRECISION: f64 = 1e-3;

    pub fn new(angle_precision: f64) -> Self {
        Self {
            cache: HashMap::new(),
            angle_precision,
        }
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
        let factor = 1.0 / self.angle_precision;
        let key = (
            a_id,
            b_id,
            (a_angle * factor).round() as i64,
            (b_angle * factor).round() as i64,
        );
        if let Some(v) = self.cache.get(&key) {
            return v.clone();
        }
        let nfp = minkowski_difference_clip(a, b);
        self.cache.insert(key, nfp.clone());
        nfp
    }
}

impl Default for NfpCache {
    fn default() -> Self {
        Self::new(Self::DEFAULT_ANGLE_PRECISION)
    }
}

/// Simple outer no-fit polygon using Minkowski difference.
pub fn no_fit_polygon(a: &[Point], b: &[Point]) -> Vec<Point> {
    minkowski_difference_clip(a, b)
}

/// Generate inner fit polygons by offsetting the container and computing the
/// outer no-fit polygon for each offset polygon.
pub fn inner_fit_polygon(container: &[Point], part: &[Point], spacing: f64) -> Vec<Vec<Point>> {
    let offsets = offset_polygon(container, -spacing.abs());
    offsets
        .into_iter()
        .flat_map(|poly| minkowski_diff_erosion(&poly, part))
        .collect()
}

/// Interior NFP when the container is an axis-aligned rectangle.
/// Returns `None` if `part` is larger than the rectangle.
pub fn no_fit_polygon_rectangle(container: &[Point], part: &[Point]) -> Option<Vec<Vec<Point>>> {
    let ab = get_polygon_bounds(container)?;
    let bb = get_polygon_bounds(part)?;

    if bb.width > ab.width || bb.height > ab.height {
        return None;
    }

    let dx1 = ab.x - bb.x + part[0].x;
    let dy1 = ab.y - bb.y + part[0].y;
    let dx2 = ab.x + ab.width - (bb.x + bb.width) + part[0].x;
    let dy2 = ab.y + ab.height - (bb.y + bb.height) + part[0].y;

    Some(vec![vec![
        Point { x: dx1, y: dy1 },
        Point { x: dx2, y: dy1 },
        Point { x: dx2, y: dy2 },
        Point { x: dx1, y: dy2 },
    ]])
}

fn to_geo_polygon(points: &[Point]) -> GeoPolygon<f64> {
    let ls: LineString<f64> = points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>().into();
    GeoPolygon::new(ls, vec![])
}

fn minkowski_diff_erosion(container: &[Point], part: &[Point]) -> Vec<Vec<Point>> {
    if container.is_empty() || part.is_empty() {
        return Vec::new();
    }
    let container_geo = to_geo_polygon(container);
    let mut acc: Option<geo_types::MultiPolygon<f64>> = None;
    for v in part {
        let shifted = container_geo.translate(-v.x, -v.y);
        let mp = geo_types::MultiPolygon(vec![shifted]);
        acc = Some(match acc {
            Some(a) => Clipper::intersection(&a, &mp, CLIPPER_SCALE),
            None => mp,
        });
    }
    let mp = acc.unwrap();
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

/// General no-fit polygon. When `inside` is `true` this computes the interior
/// no-fit polygons by offsetting the container before applying the Minkowski
/// difference. When `inside` is `false` the outer no-fit polygon is returned.
pub fn no_fit_polygon_general(
    container: &[Point],
    part: &[Point],
    inside: bool,
    spacing: f64,
) -> Vec<Vec<Point>> {
    if inside {
        inner_fit_polygon(container, part, spacing)
    } else {
        vec![minkowski_difference_clip(container, part)]
    }
}

fn multipolygon_to_polygons(mp: geo_types::MultiPolygon<f64>) -> Vec<Vec<Point>> {
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

fn polygons_to_multipolygon(polys: &[Vec<Point>]) -> geo_types::MultiPolygon<f64> {
    let mut mp = geo_types::MultiPolygon(vec![]);
    for poly in polys {
        if poly.len() < 3 {
            continue;
        }
        let g = to_geo_polygon(poly);
        mp = if mp.0.is_empty() {
            geo_types::MultiPolygon(vec![g])
        } else {
            Clipper::union(&mp, &g, CLIPPER_SCALE)
        };
    }
    mp
}

/// Union a list of polygons into a single MultiPolygon using geo_clipper.
pub fn union_polygons(polys: &[Vec<Point>]) -> Vec<Vec<Point>> {
    let mp = polygons_to_multipolygon(polys);
    multipolygon_to_polygons(mp)
}

/// Difference of subject minus clip polygons using geo_clipper.
pub fn difference_polygons(subject: &[Vec<Point>], clip: &[Vec<Point>]) -> Vec<Vec<Point>> {
    let subj_mp = polygons_to_multipolygon(subject);
    let clip_mp = polygons_to_multipolygon(clip);
    let diff = Clipper::difference(&subj_mp, &clip_mp, CLIPPER_SCALE);
    multipolygon_to_polygons(diff)
}
