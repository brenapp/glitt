use ratatui::DefaultTerminal;

pub mod rebase;

#[derive(Clone, Debug, clap::ValueEnum)]
enum EditorKind {
    Rebase,
}

pub trait Editor {
    fn render(&mut self, frame: &mut ratatui::Frame);
    fn run(&mut self, terminal: DefaultTerminal) -> color_eyre::Result<()>;
}
