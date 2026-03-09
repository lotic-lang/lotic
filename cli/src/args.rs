use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lotic", version, about = "CLI for the Lotic build system")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Build {
        // Arguments for `cargo build-sbf`
        #[clap(required = false, last = true)]
        cargo_args: Vec<String>,
    },
}
