use rand::prelude::*;
use rayon::prelude::*;

use crate::geometry::{
    Bounds, get_polygon_bounds, get_polygons_bounds, point_in_polygon,
    polygon_area, polygons_intersect, polygon_contains_polygon,
};
use crate::nfp::{self, NfpCache};
use crate::part::Part;
use crate::svg_parser::{Point, Polygon};
use anyhow::{self, Result};

#[derive(Clone, Copy)]
pub struct GAConfig {
    pub population_size: usize,
    pub mutation_rate: usize,
    pub rotations: usize,
    pub spacing: f64,
    pub use_holes: bool,
    pub explore_concave: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub idx: usize,
    pub angle: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug)]
struct FreeRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone)]
pub struct Individual {
    pub placement: Vec<usize>,
    pub rotation: Vec<f64>,
    pub fitness: f64,
}

pub struct GeneticAlgorithm<'a> {
    parts: &'a [Part],
    bin_bounds: Bounds,
    config: GAConfig,
    pub population: Vec<Individual>,
}

impl<'a> GeneticAlgorithm<'a> {
    pub fn new(parts: &'a [Part], bin: &'a Polygon, config: GAConfig) -> Result<Self> {
        let bin_bounds = get_polygon_bounds(&bin.points)
            .ok_or_else(|| anyhow::anyhow!("failed to compute bin bounds"))?;
        let mut ga = GeneticAlgorithm {
            parts,
            bin_bounds,
            config,
            population: Vec::new(),
        };
        let angles: Vec<f64> = parts.iter().map(|p| ga.random_angle(p)).collect();
        let base = Individual {
            placement: (0..parts.len()).collect(),
            rotation: angles,
            fitness: f64::MAX,
        };
        ga.population.push(base.clone());
        while ga.population.len() < config.population_size {
            let m = ga.mutate(&base);
            ga.population.push(m);
        }
        Ok(ga)
    }

    fn random_angle(&self, part: &Part) -> f64 {
        if self.config.rotations == 0 {
            return 0.0;
        }
        let mut angles: Vec<f64> = (0..self.config.rotations)
            .map(|i| i as f64 * 360.0 / self.config.rotations as f64)
            .collect();
        let mut rng = thread_rng();
        angles.shuffle(&mut rng);
        for angle in angles {
            let rotated = part.rotated(angle);
            if let Some(b) = get_polygons_bounds(&rotated) {
                if b.width <= self.bin_bounds.width && b.height <= self.bin_bounds.height {
                    return angle;
                }
            }
        }
        0.0
    }

    fn evaluate(&self, ind: &Individual) -> f64 {
        evaluate_static(ind, self.parts, self.bin_bounds, self.config)
    }

    fn mutate(&self, ind: &Individual) -> Individual {
        let mut rng = thread_rng();
        let mut placement = ind.placement.clone();
        let mut rotation = ind.rotation.clone();
        for i in 0..placement.len() {
            if rng.r#gen::<f64>() < self.config.mutation_rate as f64 * 0.01 {
                if i + 1 < placement.len() {
                    placement.swap(i, i + 1);
                }
            }
            if rng.r#gen::<f64>() < self.config.mutation_rate as f64 * 0.01 {
                rotation[i] = self.random_angle(&self.parts[placement[i]]);
            }
        }
        Individual {
            placement,
            rotation,
            fitness: f64::MAX,
        }
    }

    fn mate(&self, male: &Individual, female: &Individual) -> (Individual, Individual) {
        let len = male.placement.len();
        let mut rng = thread_rng();
        let cut = ((len as f64 * rng.gen_range(0.1..0.9)).round()) as usize;
        let mut gene1 = male.placement[..cut].to_vec();
        let mut rot1 = male.rotation[..cut].to_vec();
        for (&p, &r) in female.placement.iter().zip(&female.rotation) {
            if !gene1.contains(&p) {
                gene1.push(p);
                rot1.push(r);
            }
        }
        let mut gene2 = female.placement[..cut].to_vec();
        let mut rot2 = female.rotation[..cut].to_vec();
        for (&p, &r) in male.placement.iter().zip(&male.rotation) {
            if !gene2.contains(&p) {
                gene2.push(p);
                rot2.push(r);
            }
        }
        (
            Individual {
                placement: gene1,
                rotation: rot1,
                fitness: f64::MAX,
            },
            Individual {
                placement: gene2,
                rotation: rot2,
                fitness: f64::MAX,
            },
        )
    }

    fn random_weighted_index(&self, exclude: Option<usize>) -> usize {
        let mut rng = thread_rng();
        let mut idxs: Vec<usize> = (0..self.population.len()).collect();
        if let Some(e) = exclude {
            idxs.retain(|&v| v != e);
        }
        let rand = rng.r#gen::<f64>();
        let mut lower = 0.0;
        let weight = 1.0 / idxs.len() as f64;
        let mut upper = weight;
        for (pos, &i) in idxs.iter().enumerate() {
            if rand > lower && rand < upper {
                return i;
            }
            lower = upper;
            upper += 2.0 * weight * ((idxs.len() - pos) as f64 / idxs.len() as f64);
        }
        idxs[0]
    }

    pub fn evaluate_population(&mut self) {
        let parts = self.parts;
        let bounds = self.bin_bounds;
        let cfg = self.config;
        self.population.par_iter_mut().for_each(|ind| {
            ind.fitness = evaluate_static(ind, parts, bounds, cfg);
        });
    }

    pub fn generation(&mut self) {
        self.population.sort_by(|a, b| {
            a.fitness
                .partial_cmp(&b.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut newpop = vec![self.population[0].clone()];
        while newpop.len() < self.population.len() {
            let m_idx = self.random_weighted_index(None);
            let f_idx = self.random_weighted_index(Some(m_idx));
            let (c1, c2) = self.mate(&self.population[m_idx], &self.population[f_idx]);
            newpop.push(self.mutate(&c1));
            if newpop.len() < self.population.len() {
                newpop.push(self.mutate(&c2));
            }
        }
        self.population = newpop;
    }

    pub fn evolve(&mut self, generations: usize) {
        for _ in 0..generations {
            self.evaluate_population();
            self.generation();
        }
        self.evaluate_population();
    }

    pub fn create_svg(&self, ind: &Individual) -> String {
        // reuse the filtering logic from evaluation so that SVG output ignores
        // parts that cannot fit into the bin
        let mut placement_ids = Vec::new();
        let mut rotation = Vec::new();
        for (&idx, &angle) in ind.placement.iter().zip(&ind.rotation) {
            let rotated = self.parts[idx].rotated(angle);
            if let Some(b) = get_polygons_bounds(&rotated) {
                if b.width <= self.bin_bounds.width && b.height <= self.bin_bounds.height {
                    placement_ids.push(idx);
                    rotation.push(angle);
                }
            }
        }
        let filtered = Individual {
            placement: placement_ids,
            rotation,
            fitness: 0.0,
        };
        let (_height, placement) = layout(&filtered, self.parts, self.bin_bounds, self.config);
        let mut body = String::new();
        for p in &placement {
            let part = &self.parts[p.idx];
            let rotated = part.rotated(p.angle);
            for poly in rotated {
                let points: Vec<String> = poly
                    .points
                    .into_iter()
                    .map(|pt| format!("{},{}", pt.x + p.x, pt.y + p.y))
                    .collect();
                body.push_str(&format!(
                    "<polygon points=\"{}\" fill=\"none\" stroke=\"black\"/>\n",
                    points.join(" ")
                ));
            }
        }
        let width = self.bin_bounds.width;
        let height = _height;
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">{}<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"blue\"/></svg>",
            width, height, body, width, height
        )
    }
}

fn evaluate_static(ind: &Individual, parts: &[Part], bin_bounds: Bounds, config: GAConfig) -> f64 {
    // filter out parts that cannot possibly fit inside the bin
    let mut placement = Vec::new();
    let mut rotation = Vec::new();
    let mut unplaceable = 0usize;
    for (&idx, &angle) in ind.placement.iter().zip(&ind.rotation) {
        let part = &parts[idx];
        let rotated = part.rotated(angle);
        match get_polygons_bounds(&rotated) {
            Some(b) if b.width <= bin_bounds.width && b.height <= bin_bounds.height => {
                placement.push(idx);
                rotation.push(angle);
            }
            _ => unplaceable += 1,
        }
    }

    let filtered = Individual {
        placement,
        rotation,
        fitness: 0.0,
    };

    let (height, placed) = layout(&filtered, parts, bin_bounds, config);
    if !height.is_finite() {
        return f64::INFINITY;
    }

    // compute width used in each bin
    use std::collections::HashMap;
    let mut bin_width: HashMap<usize, f64> = HashMap::new();
    for p in &placed {
        let part = &parts[p.idx];
        if let Some(b) = get_polygons_bounds(&part.rotated(p.angle)) {
            let bin_idx = (p.y / bin_bounds.height).floor() as usize;
            let w = p.x + b.width;
            bin_width
                .entry(bin_idx)
                .and_modify(|v| {
                    if w > *v {
                        *v = w;
                    }
                })
                .or_insert(w);
        }
    }

    let bin_area = bin_bounds.width * bin_bounds.height;
    let mut fitness = bin_width.len() as f64;
    for width in bin_width.values() {
        fitness += width / bin_area;
    }
    fitness += 2.0 * unplaceable as f64;
    fitness
}

fn layout(
    ind: &Individual,
    parts: &[Part],
    bin_bounds: Bounds,
    config: GAConfig,
) -> (f64, Vec<Placement>) {
    let bin_polygon = vec![
        Point { x: 0.0, y: 0.0 },
        Point {
            x: bin_bounds.width,
            y: 0.0,
        },
        Point {
            x: bin_bounds.width,
            y: bin_bounds.height,
        },
        Point {
            x: 0.0,
            y: bin_bounds.height,
        },
    ];
    let mut nfp_cache = NfpCache::new();

    if !config.explore_concave {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut bins = 1;
        let mut placement: Vec<Placement> = Vec::new();
        for (&idx, &angle) in ind.placement.iter().zip(&ind.rotation) {
            let part = &parts[idx];
            let rotated = part.rotated(angle);
            let b = match get_polygons_bounds(&rotated) {
                Some(v) => v,
                None => continue,
            };

            if b.width > bin_bounds.width || b.height > bin_bounds.height {
                return (f64::INFINITY, Vec::new());
            }

            if x + b.width >= bin_bounds.width {
                bins += 1;
                x = 0.0;
                y += bin_bounds.height;
            }

            // bin nfp for usage (computed but not used directly)
            let _bin_nfp = nfp::inner_fit_polygon(&bin_polygon, &rotated[0].points, config.spacing);

            // check against already placed parts
            for p in &placement {
                let other_rot = parts[p.idx].rotated(p.angle);
                let orient_other = polygon_area(&other_rot[0].points).signum();
                let nfp = nfp_cache.get_or_generate(
                    p.idx,
                    idx,
                    p.angle,
                    angle,
                    &other_rot[0].points,
                    &rotated[0].points,
                );
                if nfp.len() >= 3 && point_in_polygon(&nfp, x - p.x, y - p.y) {
                    return (f64::INFINITY, Vec::new());
                }
                for op in &other_rot {
                    if polygon_area(&op.points).signum() != orient_other {
                        continue; // hole
                    }
                    for rp in &rotated {
                        if polygons_intersect(
                            &op.points,
                            &rp.points,
                            p.x,
                            p.y,
                            x,
                            y,
                        ) {
                            let mut in_hole = false;
                            for hole in &other_rot {
                                if polygon_area(&hole.points).signum() == orient_other {
                                    continue;
                                }
                                if polygon_contains_polygon(&hole.points, &rp.points, p.x, p.y, x, y) {
                                    in_hole = true;
                                    break;
                                }
                            }
                            if !in_hole {
                                return (f64::INFINITY, Vec::new());
                            }
                        }
                    }
                }
            }

            placement.push(Placement { idx, angle, x, y });
            x += b.width + config.spacing;
        }
        (bin_bounds.height * bins as f64, placement)
    } else {
        let mut bins = 1usize;
        let mut free = vec![FreeRect {
            x: 0.0,
            y: 0.0,
            width: bin_bounds.width,
            height: bin_bounds.height,
        }];
        let mut placement: Vec<Placement> = Vec::new();
        for (&idx, &angle) in ind.placement.iter().zip(&ind.rotation) {
            let part = &parts[idx];
            let rotated = part.rotated(angle);
            let b = match get_polygons_bounds(&rotated) {
                Some(v) => v,
                None => continue,
            };

            if b.width > bin_bounds.width || b.height > bin_bounds.height {
                return (f64::INFINITY, Vec::new());
            }

            loop {
                let mut placed = false;
                for i in 0..free.len() {
                    let rect = free[i];
                    if b.width <= rect.width && b.height <= rect.height {
                        let x = rect.x;
                        let y = rect.y;

                        // compute bin nfp (not used directly)
                        let _ = nfp::inner_fit_polygon(
                            &bin_polygon,
                            &rotated[0].points,
                            config.spacing,
                        );

                        let mut collide = false;
                        for p in &placement {
                            let other_rot = parts[p.idx].rotated(p.angle);
                            let orient_other = polygon_area(&other_rot[0].points).signum();
                            let nfp = nfp_cache.get_or_generate(
                                p.idx,
                                idx,
                                p.angle,
                                angle,
                                &other_rot[0].points,
                                &rotated[0].points,
                            );
                            if nfp.len() >= 3 && point_in_polygon(&nfp, x - p.x, y - p.y) {
                                collide = true;
                                break;
                            }
                            for op in &other_rot {
                                if polygon_area(&op.points).signum() != orient_other {
                                    continue;
                                }
                                for rp in &rotated {
                                    if polygons_intersect(
                                        &op.points,
                                        &rp.points,
                                        p.x,
                                        p.y,
                                        x,
                                        y,
                                    ) {
                                        let mut in_hole = false;
                                        for hole in &other_rot {
                                            if polygon_area(&hole.points).signum() == orient_other {
                                                continue;
                                            }
                                            if polygon_contains_polygon(&hole.points, &rp.points, p.x, p.y, x, y) {
                                                in_hole = true;
                                                break;
                                            }
                                        }
                                        if !in_hole {
                                            collide = true;
                                            break;
                                        }
                                    }
                                }
                                if collide {
                                    break;
                                }
                            }
                            if collide {
                                break;
                            }
                        }
                        if collide {
                            continue;
                        }

                        placement.push(Placement { idx, angle, x, y });
                        free.remove(i);
                        let right_w = rect.width - b.width - config.spacing;
                        if right_w > 0.0 {
                            free.push(FreeRect {
                                x: x + b.width + config.spacing,
                                y,
                                width: right_w,
                                height: b.height,
                            });
                        }
                        let bottom_h = rect.height - b.height - config.spacing;
                        if bottom_h > 0.0 {
                            free.push(FreeRect {
                                x,
                                y: y + b.height + config.spacing,
                                width: rect.width,
                                height: bottom_h,
                            });
                        }
                        if config.use_holes {
                            let orient = polygon_area(&rotated[0].points).signum();
                            for poly in rotated.iter().skip(1) {
                                let area = polygon_area(&poly.points);
                                if orient != 0.0 && area.signum() != orient {
                                    if let Some(hb) = get_polygon_bounds(&poly.points) {
                                        free.insert(
                                            0,
                                            FreeRect {
                                                x: x + hb.x,
                                                y: y + hb.y,
                                                width: hb.width,
                                                height: hb.height,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        placed = true;
                        break;
                    }
                }
                if placed {
                    break;
                }
                let start_y = bin_bounds.height * bins as f64;
                free.push(FreeRect {
                    x: 0.0,
                    y: start_y,
                    width: bin_bounds.width,
                    height: bin_bounds.height,
                });
                bins += 1;
            }
        }
        (bin_bounds.height * bins as f64, placement)
    }
}
