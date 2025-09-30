use anyhow::Result;
use clap::{Parser, Subcommand};

mod generator;
mod finder_cpu;

#[derive(Parser)]
#[command(name = "seed-recovery")]
#[command(about = "Seed recovery tool for BIP39 mnemonic phrases")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate seed combinations from known word positions
    Generate {
        /// Path to generator config file
        config: String,
    },
    /// Find seed that matches target address
    Find {
        /// Path to finder config file
        config: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { config } => {
            generator::run_generator(&config)
        }
        Commands::Find { config } => {
            finder_cpu::run_finder(&config)
        }
    }
}