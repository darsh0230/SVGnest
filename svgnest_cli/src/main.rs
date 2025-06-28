use clap::Parser;
use std::path::PathBuf;

mod dxf_parser;
mod ga;
mod geometry;
mod line_merge;
mod part;
mod svg_parser;

/// Command line arguments for SVGnest
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct CliArgs {
    /// SVG input files to be nested
    #[arg(long, value_name = "FILES", required = true)]
    pub inputs: Vec<PathBuf>,

    /// Maximum error allowed when approximating curves
    #[arg(long = "approx-tolerance", default_value_t = 0.3)]
    pub approx_tolerance: f64,

    /// Minimum space between parts
    #[arg(long, default_value_t = 0.0)]
    pub spacing: f64,

    /// Number of rotations to test for each part
    #[arg(long, default_value_t = 4)]
    pub rotations: usize,

    /// Population size for the genetic algorithm
    #[arg(long, default_value_t = 10, value_name = "SIZE")]
    pub population_size: usize,

    /// Mutation rate of the genetic algorithm (1-50)
    #[arg(long, default_value_t = 10, value_name = "RATE")]
    pub mutation_rate: usize,

    /// Place parts inside the holes of other parts
    #[arg(long, default_value_t = false)]
    pub use_holes: bool,

    /// Explore concave areas for more robust placement
    #[arg(long, default_value_t = false)]
    pub explore_concave: bool,

    /// Merge overlapping line segments
    #[arg(long, default_value_t = false)]
    pub merge_lines: bool,
}

/// Parsed configuration returned by the CLI
#[derive(Debug)]
pub struct Config {
    pub inputs: Vec<PathBuf>,
    pub approx_tolerance: f64,
    pub spacing: f64,
    pub rotations: usize,
    pub population_size: usize,
    pub mutation_rate: usize,
    pub use_holes: bool,
    pub explore_concave: bool,
    pub merge_lines: bool,
}

impl From<CliArgs> for Config {
    fn from(args: CliArgs) -> Self {
        Self {
            inputs: args.inputs,
            approx_tolerance: args.approx_tolerance,
            spacing: args.spacing,
            rotations: args.rotations,
            population_size: args.population_size,
            mutation_rate: args.mutation_rate,
            use_holes: args.use_holes,
            explore_concave: args.explore_concave,
            merge_lines: args.merge_lines,
        }
    }
}

/// Parse command line arguments into a configuration struct
pub fn parse_config() -> Config {
    let args = CliArgs::parse();
    args.into()
}

fn main() {
    let cfg = parse_config();

    let mut parts = Vec::new();
    let mut bin: Option<svg_parser::Polygon> = None;
    for path in &cfg.inputs {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let res = if ext.eq_ignore_ascii_case("dxf") {
            dxf_parser::part_from_dxf(path)
        } else {
            svg_parser::polygons_from_file(path, cfg.merge_lines, cfg.approx_tolerance)
                .map(|p| crate::part::Part::new(p))
        };
        match res {
            Ok(p) => {
                if bin.is_none() {
                    bin = p.polygons.first().cloned();
                } else {
                    parts.push(p);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse {}: {}", path.display(), e);
                return;
            }
        }
    }

    let bin = match bin {
        Some(b) => b,
        None => {
            eprintln!("No polygons found in input");
            return;
        }
    };

    if parts.is_empty() {
        eprintln!("No polygons found in input");
        return;
    }

    let ga_cfg = ga::GAConfig {
        population_size: cfg.population_size,
        mutation_rate: cfg.mutation_rate,
        rotations: cfg.rotations,
        spacing: cfg.spacing,
    };
    let mut ga = match ga::GeneticAlgorithm::new(&parts, &bin, ga_cfg) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to initialize algorithm: {}", e);
            return;
        }
    };
    ga.evolve(10);
    let best = match ga.population.iter().min_by(|a, b| {
        a.fitness
            .partial_cmp(&b.fitness)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        Some(v) => v,
        None => {
            eprintln!("No population available to evaluate");
            return;
        }
    };
    let svg = ga.create_svg(best);
    if let Err(e) = std::fs::write("nested.svg", svg) {
        eprintln!("Failed to write SVG: {}", e);
        return;
    }
    println!("Nested result written to nested.svg");
}
