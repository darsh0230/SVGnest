use crate::{
    geometry::{
        normalize_polygons, Bounds, get_polygons_bounds, rotate_polygon,
    },
    svg_parser::Polygon,
};

#[derive(Debug, Clone)]
pub struct Part {
    pub polygons: Vec<Polygon>,
}

impl Part {
    pub fn new(polys: Vec<Polygon>) -> Self {
        let mut p = polys;
        normalize_polygons(&mut p);
        Self { polygons: p }
    }

    pub fn rotated(&self, angle: f64) -> Vec<Polygon> {
        let mut result: Vec<Polygon> = self
            .polygons
            .iter()
            .map(|p| Polygon {
                id: p.id,
                points: rotate_polygon(&p.points, angle),
                closed: p.closed,
            })
            .collect();
        normalize_polygons(&mut result);
        result
    }

    pub fn bounds(&self) -> Option<Bounds> {
        get_polygons_bounds(&self.polygons)
    }

    pub fn bounds_rotated(&self, angle: f64) -> Option<Bounds> {
        let rot = self.rotated(angle);
        get_polygons_bounds(&rot)
    }
}
