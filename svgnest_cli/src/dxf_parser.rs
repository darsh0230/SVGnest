#[cfg(feature = "dxf")]
use dxf::{Drawing, entities::EntityType};
use std::path::Path;

use crate::svg_parser::{Point, Polygon};

#[cfg(feature = "dxf")]
fn approximate_arc(cx: f64, cy: f64, r: f64, start: f64, end: f64, segments: usize) -> Vec<Point> {
    let mut pts = Vec::new();
    let step = (end - start) / segments as f64;
    for i in 0..=segments {
        let a = start + step * i as f64;
        pts.push(Point { x: cx + r * a.cos(), y: cy + r * a.sin() });
    }
    pts
}

#[cfg(feature = "dxf")]
fn approximate_ellipse(
    center: &dxf::Point,
    major: &dxf::Vector,
    normal: &dxf::Vector,
    ratio: f64,
    start: f64,
    end: f64,
    segments: usize,
) -> Vec<Point> {
    let major_len = (major.x * major.x + major.y * major.y + major.z * major.z).sqrt();
    if major_len == 0.0 {
        return Vec::new();
    }
    let ux = major.x / major_len;
    let uy = major.y / major_len;
    let uz = major.z / major_len;
    let nx = normal.x;
    let ny = normal.y;
    let nz = normal.z;
    // v = normal x u
    let mut vx = ny * uz - nz * uy;
    let mut vy = nz * ux - nx * uz;
    let mut vz = nx * uy - ny * ux;
    let v_len = (vx * vx + vy * vy + vz * vz).sqrt();
    if v_len != 0.0 {
        vx /= v_len;
        vy /= v_len;
        vz /= v_len;
    }

    let a = major_len;
    let b = a * ratio;
    let step = (end - start) / segments as f64;
    let mut pts = Vec::new();
    for i in 0..=segments {
        let t = start + step * i as f64;
        let cos_t = t.cos();
        let sin_t = t.sin();
        let x = center.x + a * ux * cos_t + b * vx * sin_t;
        let y = center.y + a * uy * cos_t + b * vy * sin_t;
        pts.push(Point { x, y });
    }
    pts
}

#[cfg(feature = "dxf")]
fn approximate_bulge(p1: &Point, p2: &Point, bulge: f64, segments: usize) -> Vec<Point> {
    if segments == 0 {
        return vec![*p1, *p2];
    }
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let chord = (dx * dx + dy * dy).sqrt();
    if chord == 0.0 {
        return vec![*p1];
    }
    let theta = 4.0 * bulge.atan();
    let r = chord / (2.0 * (theta / 2.0).sin());
    let mx = (p1.x + p2.x) / 2.0;
    let my = (p1.y + p2.y) / 2.0;
    let d = (r * r - (chord / 2.0).powi(2)).abs().sqrt();
    let sign = bulge.signum();
    let ux = -dy / chord;
    let uy = dx / chord;
    let cx = mx + sign * ux * d;
    let cy = my + sign * uy * d;
    let mut start_ang = (p1.y - cy).atan2(p1.x - cx);
    let mut end_ang = (p2.y - cy).atan2(p2.x - cx);
    if sign > 0.0 && end_ang < start_ang {
        end_ang += std::f64::consts::TAU;
    } else if sign < 0.0 && end_ang > start_ang {
        end_ang -= std::f64::consts::TAU;
    }
    let step = (end_ang - start_ang) / segments as f64;
    let mut pts = Vec::new();
    for i in 0..=segments {
        let a = start_ang + step * i as f64;
        pts.push(Point { x: cx + r * a.cos(), y: cy + r * a.sin() });
    }
    pts
}

#[cfg(feature = "dxf")]
pub fn polygons_from_dxf(path: &Path) -> anyhow::Result<Vec<Polygon>> {
    let drawing = Drawing::load_file(path)?;
    let mut polys = Vec::new();
    for e in drawing.entities() {
        match &e.specific {
            EntityType::Line(line) => {
                polys.push(Polygon {
                    id: 0,
                    points: vec![
                        Point {
                            x: line.p1.x,
                            y: line.p1.y,
                        },
                        Point {
                            x: line.p2.x,
                            y: line.p2.y,
                        },
                    ],
                    closed: false,
                });
            }
            EntityType::LwPolyline(poly) => {
                let mut pts = Vec::new();
                let vtx = &poly.vertices;
                if !vtx.is_empty() {
                    for i in 0..vtx.len() {
                        let curr = &vtx[i];
                        let next_idx = if i + 1 < vtx.len() {
                            i + 1
                        } else if poly.is_closed() {
                            0
                        } else {
                            pts.push(Point { x: curr.x, y: curr.y });
                            continue;
                        };
                        let next = &vtx[next_idx];
                        let p1 = Point { x: curr.x, y: curr.y };
                        let p2 = Point { x: next.x, y: next.y };
                        if curr.bulge.abs() > f64::EPSILON {
                            let theta = 4.0 * curr.bulge.atan();
                            let segs = ((theta.abs() / std::f64::consts::TAU) * 32.0).ceil() as usize;
                            let arc = approximate_bulge(&p1, &p2, curr.bulge, segs.max(1));
                            if pts.last().map_or(true, |p| p.x != p1.x || p.y != p1.y) {
                                pts.push(p1);
                            }
                            pts.extend_from_slice(&arc[1..]);
                        } else {
                            pts.push(p1);
                        }
                    }
                    if !poly.is_closed() {
                        if let Some(last) = vtx.last() {
                            pts.push(Point { x: last.x, y: last.y });
                        }
                    }
                    polys.push(Polygon { id: 0, points: pts, closed: poly.is_closed() });
                }
            }
            EntityType::Polyline(poly) => {
                let verts: Vec<_> = poly.vertices().cloned().collect();
                if !verts.is_empty() {
                    let mut pts = Vec::new();
                    for i in 0..verts.len() {
                        let curr = &verts[i];
                        let next_idx = if i + 1 < verts.len() {
                            i + 1
                        } else if poly.is_closed() {
                            0
                        } else {
                            pts.push(Point { x: curr.location.x, y: curr.location.y });
                            continue;
                        };
                        let next = &verts[next_idx];
                        let p1 = Point { x: curr.location.x, y: curr.location.y };
                        let p2 = Point { x: next.location.x, y: next.location.y };
                        if curr.bulge.abs() > f64::EPSILON {
                            let theta = 4.0 * curr.bulge.atan();
                            let segs = ((theta.abs() / std::f64::consts::TAU) * 32.0).ceil() as usize;
                            let arc = approximate_bulge(&p1, &p2, curr.bulge, segs.max(1));
                            if pts.last().map_or(true, |p| p.x != p1.x || p.y != p1.y) {
                                pts.push(p1);
                            }
                            pts.extend_from_slice(&arc[1..]);
                        } else {
                            pts.push(p1);
                        }
                    }
                    if !poly.is_closed() {
                        if let Some(last) = verts.last() {
                            pts.push(Point { x: last.location.x, y: last.location.y });
                        }
                    }
                    polys.push(Polygon { id: 0, points: pts, closed: poly.is_closed() });
                }
            }
            EntityType::Circle(c) => {
                let segments = 32;
                let mut pts = Vec::new();
                for i in 0..segments {
                    let theta = i as f64 * std::f64::consts::TAU / segments as f64;
                    let x = c.center.x + c.radius * theta.cos();
                    let y = c.center.y + c.radius * theta.sin();
                    pts.push(Point { x, y });
                }
                polys.push(Polygon {
                    id: 0,
                    points: pts,
                    closed: true,
                });
            }
            EntityType::Arc(arc) => {
                let mut end = arc.end_angle - arc.start_angle;
                if end <= 0.0 {
                    end += 360.0;
                }
                let segs = ((end / 360.0) * 32.0).ceil() as usize;
                let pts = approximate_arc(
                    arc.center.x,
                    arc.center.y,
                    arc.radius,
                    arc.start_angle.to_radians(),
                    (arc.start_angle + end).to_radians(),
                    segs.max(1),
                );
                polys.push(Polygon { id: 0, points: pts, closed: false });
            }
            EntityType::Ellipse(el) => {
                let mut end = el.end_parameter - el.start_parameter;
                if end <= 0.0 {
                    end += std::f64::consts::TAU;
                }
                let segs = ((end / std::f64::consts::TAU) * 32.0).ceil() as usize;
                let pts = approximate_ellipse(
                    &el.center,
                    &el.major_axis,
                    &el.normal,
                    el.minor_axis_ratio,
                    el.start_parameter,
                    el.start_parameter + end,
                    segs.max(1),
                );
                polys.push(Polygon { id: 0, points: pts, closed: false });
            }
            _ => {}
        }
    }
    for (i, p) in polys.iter_mut().enumerate() {
        p.id = i;
    }
    Ok(polys)
}

#[cfg(not(feature = "dxf"))]
pub fn polygons_from_dxf(_path: &Path) -> anyhow::Result<Vec<Polygon>> {
    Err(anyhow::anyhow!("DXF support not enabled"))
}
