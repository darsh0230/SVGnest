use std::path::PathBuf;
use clap::Parser;

/// Simple command line interface for SVGnest
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to a configuration file in JSON format
    #[arg(short, long)]
    config: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    if let Some(cfg) = args.config {
        println!("Using config: {}", cfg.display());
    } else {
        println!("No configuration provided");
    }
}
