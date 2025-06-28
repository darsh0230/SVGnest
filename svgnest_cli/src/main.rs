use clap::Parser;
use std::path::PathBuf;

mod svg_parser;
mod geometry;

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
    println!("{:?}", cfg);
}
