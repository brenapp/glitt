use clap::Parser;
use color_eyre::Result;
mod editors;

use crate::editors::Editor;

#[derive(clap::Subcommand)]
pub enum Commands {
    Rebase {
        /// Path to the rebase todo file
        path: std::path::PathBuf,
    },
}

#[derive(clap::Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    let args = Cli::try_parse()?;
    let terminal = ratatui::init();

    match args.command {
        Commands::Rebase { path } => {
            let cwd = std::env::current_dir()?;
            let path = if path.is_absolute() {
                path
            } else {
                cwd.join(path).canonicalize()?
            };

            let mut editor = editors::rebase::RebaseEditor::new(path)?;
            editor.run(terminal)
        }
    }
}
