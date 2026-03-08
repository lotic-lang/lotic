use {
    camino::Utf8PathBuf,
    clap::{Parser, Subcommand},
};

#[derive(Parser)]
#[command(name = "lotic", version, about = "CLI for the Lotic build system")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Build {
        // Path to the Cargo.toml file
        #[arg(long, default_value = "Cargo.toml")]
        manifest_path: Utf8PathBuf,
        // Arguments for `cargo build-sbf`
        #[clap(required = false, last = true)]
        cargo_args: Vec<String>
    },
}
