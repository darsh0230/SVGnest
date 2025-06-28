#[cfg(feature = "dxf")]
use dxf::{Drawing, entities::EntityType};
use std::path::Path;

use crate::{
    part::Part,
    svg_parser::{Point, Polygon},
};

const CONNECT_TOLERANCE: f64 = 1e-6;

fn points_equal(a: &Point, b: &Point) -> bool {
    (a.x - b.x).abs() < CONNECT_TOLERANCE && (a.y - b.y).abs() < CONNECT_TOLERANCE
}

#[cfg(feature = "dxf")]
fn approximate_arc(cx: f64, cy: f64, r: f64, start: f64, end: f64, segments: usize) -> Vec<Point> {
    let mut pts = Vec::new();
    let step = (end - start) / segments as f64;
    for i in 0..=segments {
        let a = start + step * i as f64;
        pts.push(Point {
            x: cx + r * a.cos(),
            y: cy + r * a.sin(),
        });
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
        pts.push(Point {
            x: cx + r * a.cos(),
            y: cy + r * a.sin(),
        });
    }
    pts
}

fn connect_open_polys(mut open: Vec<Vec<Point>>, mut closed: Vec<Polygon>) -> Vec<Polygon> {
    while let Some(mut current) = open.pop() {
        let mut changed = true;
        while changed {
            changed = false;
            let mut i = 0;
            while i < open.len() {
                let other = &open[i];
                let first_cur = current.first().unwrap();
                let last_cur = current.last().unwrap();
                let first_other = other.first().unwrap();
                let last_other = other.last().unwrap();

                if points_equal(last_cur, first_other) {
                    current.extend(other.iter().skip(1).cloned());
                    open.remove(i);
                    changed = true;
                } else if points_equal(last_cur, last_other) {
                    current.extend(other.iter().rev().skip(1).cloned());
                    open.remove(i);
                    changed = true;
                } else if points_equal(first_cur, last_other) {
                    let mut add: Vec<Point> = other.iter().rev().skip(1).cloned().collect();
                    add.extend(current);
                    current = add;
                    open.remove(i);
                    changed = true;
                } else if points_equal(first_cur, first_other) {
                    let mut add: Vec<Point> = other.iter().skip(1).rev().cloned().collect();
                    add.extend(current);
                    current = add;
                    open.remove(i);
                    changed = true;
                } else {
                    i += 1;
                }
                if changed {
                    break;
                }
            }
        }

        let is_closed = points_equal(current.first().unwrap(), current.last().unwrap());
        if is_closed && current.len() > 1 {
            current.pop();
        }
        closed.push(Polygon {
            id: 0,
            points: current,
            closed: is_closed,
        });
    }
    closed
}

#[cfg(feature = "dxf")]
pub fn part_from_dxf(path: &Path) -> anyhow::Result<Part> {
    let drawing = Drawing::load_file(path)?;
    let mut open = Vec::new();
    let mut closed = Vec::new();
    for e in drawing.entities() {
        match &e.specific {
            EntityType::Line(line) => {
                open.push(vec![
                    Point {
                        x: line.p1.x,
                        y: line.p1.y,
                    },
                    Point {
                        x: line.p2.x,
                        y: line.p2.y,
                    },
                ]);
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
                            pts.push(Point {
                                x: curr.x,
                                y: curr.y,
                            });
                            continue;
                        };
                        let next = &vtx[next_idx];
                        let p1 = Point {
                            x: curr.x,
                            y: curr.y,
                        };
                        let p2 = Point {
                            x: next.x,
                            y: next.y,
                        };
                        if curr.bulge.abs() > f64::EPSILON {
                            let theta = 4.0 * curr.bulge.atan();
                            let segs =
                                ((theta.abs() / std::f64::consts::TAU) * 32.0).ceil() as usize;
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
                            pts.push(Point {
                                x: last.x,
                                y: last.y,
                            });
                        }
                        open.push(pts);
                    } else {
                        closed.push(Polygon {
                            id: 0,
                            points: pts,
                            closed: true,
                        });
                    }
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
                            pts.push(Point {
                                x: curr.location.x,
                                y: curr.location.y,
                            });
                            continue;
                        };
                        let next = &verts[next_idx];
                        let p1 = Point {
                            x: curr.location.x,
                            y: curr.location.y,
                        };
                        let p2 = Point {
                            x: next.location.x,
                            y: next.location.y,
                        };
                        if curr.bulge.abs() > f64::EPSILON {
                            let theta = 4.0 * curr.bulge.atan();
                            let segs =
                                ((theta.abs() / std::f64::consts::TAU) * 32.0).ceil() as usize;
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
                            pts.push(Point {
                                x: last.location.x,
                                y: last.location.y,
                            });
                        }
                        open.push(pts);
                    } else {
                        closed.push(Polygon {
                            id: 0,
                            points: pts,
                            closed: true,
                        });
                    }
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
                closed.push(Polygon {
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
                open.push(pts);
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
                open.push(pts);
            }
            _ => {}
        }
    }
    let mut all = connect_open_polys(open, closed);
    for (i, p) in all.iter_mut().enumerate() {
        p.id = i;
    }
    Ok(Part::new(all))
}

#[cfg(not(feature = "dxf"))]
pub fn part_from_dxf(_path: &Path) -> anyhow::Result<Part> {
    Err(anyhow::anyhow!("DXF support not enabled"))
}
