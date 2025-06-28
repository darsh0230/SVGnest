#[cfg(feature = "dxf")]
use dxf::{Drawing, entities::EntityType};
use std::path::Path;

use crate::svg_parser::{Point, Polygon};

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
                let points: Vec<Point> = poly
                    .vertices
                    .iter()
                    .map(|v| Point { x: v.x, y: v.y })
                    .collect();
                if !points.is_empty() {
                    polys.push(Polygon {
                        id: 0,
                        points,
                        closed: poly.is_closed(),
                    });
                }
            }
            EntityType::Polyline(poly) => {
                let points: Vec<Point> = poly
                    .vertices()
                    .map(|v| Point {
                        x: v.location.x,
                        y: v.location.y,
                    })
                    .collect();
                if !points.is_empty() {
                    polys.push(Polygon {
                        id: 0,
                        points,
                        closed: poly.is_closed(),
                    });
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
