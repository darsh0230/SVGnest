use crate::{
    geometry::{Bounds, get_polygons_bounds, rotate_polygon},
    svg_parser::Polygon,
};

#[derive(Debug, Clone)]
pub struct Part {
    pub polygons: Vec<Polygon>,
}

impl Part {
    pub fn new(polys: Vec<Polygon>) -> Self {
        Self { polygons: polys }
    }

    pub fn rotated(&self, angle: f64) -> Vec<Polygon> {
        self.polygons
            .iter()
            .map(|p| Polygon {
                id: p.id,
                points: rotate_polygon(&p.points, angle),
                closed: p.closed,
            })
            .collect()
    }

    pub fn bounds(&self) -> Option<Bounds> {
        get_polygons_bounds(&self.polygons)
    }

    pub fn bounds_rotated(&self, angle: f64) -> Option<Bounds> {
        let rot = self.rotated(angle);
        get_polygons_bounds(&rot)
    }
}
