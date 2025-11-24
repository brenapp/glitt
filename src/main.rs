use std::process::Command;

use clap::Parser;
use color_eyre::Result;
mod editors;

use crate::editors::{Editor, rebase::RebaseEditor};

#[derive(Clone, clap::ValueEnum)]
pub enum Commands {
    Rebase,
}

#[derive(clap::Parser)]
struct Cli {
    /// Path to edit
    path: std::path::PathBuf,

    /// The fallback editor to use.
    #[clap(long, default_value = "vim")]
    fallback: String,
}

fn main() -> Result<()> {
    let args = Cli::try_parse()?;
    let terminal = ratatui::init();

    let cwd = std::env::current_dir()?;
    let path = if args.path.is_absolute() {
        args.path
    } else {
        cwd.join(args.path).canonicalize()?
    };

    let result = if RebaseEditor::should_run(&path) {
        let mut editor = RebaseEditor::new(path)?;
        editor.run(terminal)
    } else {
        Command::new(args.fallback)
            .arg(&path)
            .status()
            .map(|status| {
                if status.success() {
                    Ok(())
                } else {
                    Err(color_eyre::eyre::eyre!(
                        "Vim exited with non-zero status: {}",
                        status
                    ))
                }
            })?
    };

    ratatui::restore();
    result
}
