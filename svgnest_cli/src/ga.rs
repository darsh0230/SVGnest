use rand::prelude::*;
use rayon::prelude::*;

use crate::geometry::{
    Bounds, get_polygon_bounds, get_polygons_bounds, rotate_polygon, rotate_polygons,
};
use crate::part::Part;
use crate::svg_parser::Polygon;
use anyhow::{self, Result};

#[derive(Clone, Copy)]
pub struct GAConfig {
    pub population_size: usize,
    pub mutation_rate: usize,
    pub rotations: usize,
    pub spacing: f64,
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
        let mut x = 0.0;
        let mut y = 0.0;
        let mut bin = 1;
        let mut body = String::new();
        for (&idx, &angle) in ind.placement.iter().zip(&ind.rotation) {
            let part = &self.parts[idx];
            let rotated = part.rotated(angle);
            let b = match get_polygons_bounds(&rotated) {
                Some(v) => v,
                None => continue,
            };
            if x + b.width > self.bin_bounds.width {
                bin += 1;
                x = 0.0;
                y += self.bin_bounds.height;
            }
            for poly in rotated {
                let points: Vec<String> = poly
                    .points
                    .into_iter()
                    .map(|p| format!("{},{}", p.x + x, p.y + y))
                    .collect();
                body.push_str(&format!(
                    "<polygon points=\"{}\" fill=\"none\" stroke=\"black\"/>\n",
                    points.join(" ")
                ));
            }
            x += b.width + self.config.spacing;
        }
        let width = self.bin_bounds.width;
        let height = self.bin_bounds.height * bin as f64;
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">{}<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"blue\"/></svg>",
            width, height, body, width, height
        )
    }
}

fn evaluate_static(ind: &Individual, parts: &[Part], bin_bounds: Bounds, config: GAConfig) -> f64 {
    let mut x = 0.0;
    let mut bins = 1;
    for (&idx, &angle) in ind.placement.iter().zip(ind.rotation.iter()) {
        let part = &parts[idx];
        let rotated = part.rotated(angle);
        let b = match get_polygons_bounds(&rotated) {
            Some(v) => v,
            None => continue,
        };
        if b.width > bin_bounds.width || b.height > bin_bounds.height {
            return f64::INFINITY;
        }
        if x + b.width > bin_bounds.width {
            bins += 1;
            x = 0.0;
        }
        x += b.width + config.spacing;
    }
    bins as f64 * bin_bounds.width + x
}
