use clap::Parser;
use color_eyre::Result;
mod editors;

use crate::editors::Editor;

#[derive(Clone, clap::ValueEnum)]
pub enum Commands {
    Rebase,
}

#[derive(clap::Parser)]
struct Cli {
    #[arg(short, long, global = true)]
    command: Option<Commands>,

    /// Path to the rebase todo file
    path: std::path::PathBuf,
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

    let mut editor: Box<dyn Editor> = match args.command {
        Some(Commands::Rebase) | None => Box::new(editors::rebase::RebaseEditor::new(path)?),
    };
    let result = editor.run(terminal);

    ratatui::restore();
    result
}
