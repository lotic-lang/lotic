use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lotic", version, about = "CLI for the Lotic build system")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new project with a name
    Init {
        /// The name of the new project
        project_name: String,
    },
    /// Build solana program
    Build {
        /// Arguments for `cargo build-sbf`
        #[clap(required = false, last = true)]
        cargo_args: Vec<String>,
    },
}
