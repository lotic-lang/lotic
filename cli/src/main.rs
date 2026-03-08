use {
    args::{Cli, Commands},
    clap::Parser,
};

mod args;
mod command_processor;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { manifest_path, cargo_args } => {
            command_processor::run_build(manifest_path, cargo_args)?;
        }
    }

    Ok(())
}
