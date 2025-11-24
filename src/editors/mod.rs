use std::path::Path;

use ratatui::DefaultTerminal;

pub mod rebase;

#[derive(Clone, Debug, clap::ValueEnum)]
enum EditorKind {
    Rebase,
}

pub trait Editor {
    /// Determine if the editor should be used for the given path
    fn should_run(path: &Path) -> bool;

    fn render(&mut self, frame: &mut ratatui::Frame);
    fn run(&mut self, terminal: DefaultTerminal) -> color_eyre::Result<()>;
}
