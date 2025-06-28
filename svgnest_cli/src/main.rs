use clap::Parser;
use std::path::PathBuf;

mod ga;
mod geometry;
mod svg_parser;

/// Command line arguments for SVGnest
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct CliArgs {
    /// SVG input files to be nested
    #[arg(long, value_name = "FILES", required = true)]
    pub inputs: Vec<PathBuf>,

    /// Maximum error allowed when approximating curves
    #[arg(long, default_value_t = 0.3)]
    pub curve_tolerance: f64,

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
}

/// Parsed configuration returned by the CLI
#[derive(Debug)]
pub struct Config {
    pub inputs: Vec<PathBuf>,
    pub curve_tolerance: f64,
    pub spacing: f64,
    pub rotations: usize,
    pub population_size: usize,
    pub mutation_rate: usize,
    pub use_holes: bool,
    pub explore_concave: bool,
}

impl From<CliArgs> for Config {
    fn from(args: CliArgs) -> Self {
        Self {
            inputs: args.inputs,
            curve_tolerance: args.curve_tolerance,
            spacing: args.spacing,
            rotations: args.rotations,
            population_size: args.population_size,
            mutation_rate: args.mutation_rate,
            use_holes: args.use_holes,
            explore_concave: args.explore_concave,
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

    let mut all_polys = Vec::new();
    for path in &cfg.inputs {
        match svg_parser::polygons_from_file(path) {
            Ok(mut p) => all_polys.append(&mut p),
            Err(e) => {
                eprintln!("Failed to parse {}: {}", path.display(), e);
                return;
            }
        }
    }

    if all_polys.is_empty() {
        eprintln!("No polygons found in input");
        return;
    }

    let bin = all_polys.remove(0);
    let ga_cfg = ga::GAConfig {
        population_size: cfg.population_size,
        mutation_rate: cfg.mutation_rate,
        rotations: cfg.rotations,
        spacing: cfg.spacing,
    };
    let mut ga = ga::GeneticAlgorithm::new(&all_polys, &bin, ga_cfg);
    ga.evolve(10);
    let best = ga
        .population
        .iter()
        .min_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
        .unwrap();
    let svg = ga.create_svg(best);
    std::fs::write("nested.svg", svg).expect("write svg");
    println!("Nested result written to nested.svg");
}
