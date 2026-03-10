use {
    args::{Cli, Commands},
    clap::Parser,
};

mod args;
mod command_processor;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { cargo_args } => {
            command_processor::run_build(cargo_args)?;
        }
        Commands::Init { project_name } => {
            command_processor::run_init(project_name)?;
        }
    }

    Ok(())
}
