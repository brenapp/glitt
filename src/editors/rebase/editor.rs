use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    layout::{Constraint, Layout},
    text::ToLine,
    widgets::Paragraph,
};

use crate::editors::{
    Editor,
    rebase::todo::{RebaseTodo, RebaseTodoLine},
};
use std::path::PathBuf;

pub struct RebaseEditor {
    path: PathBuf,
    line: usize,
    todo: RebaseTodo,
}

impl RebaseEditor {
    pub fn new(path: PathBuf) -> Result<Self, color_eyre::Report> {
        let content = std::fs::read_to_string(&path)?;
        let todo = RebaseTodo::parse(&content);

        let line = todo
            .lines()
            .iter()
            .position(|line| !matches!(line, RebaseTodoLine::Comment { .. }))
            .unwrap_or(0);

        Ok(Self { path, todo, line })
    }

    pub fn move_cursor_down(&mut self) {
        let lines = self.todo.lines();
        let len = lines.len();
        if len == 0 {
            return;
        }

        let mut idx = self.line;
        for _ in 0..len {
            idx = (idx + 1) % len;
            if !matches!(lines[idx], RebaseTodoLine::Comment { .. }) {
                self.line = idx;
                return;
            }
        }
    }

    pub fn move_cursor_up(&mut self) {
        let lines = self.todo.lines();
        let len = lines.len();
        if len == 0 {
            return;
        }

        let mut idx = self.line;
        for _ in 0..len {
            if idx == 0 {
                idx = len - 1;
            } else {
                idx -= 1;
            }
            if !matches!(lines[idx], RebaseTodoLine::Comment { .. }) {
                self.line = idx;
                return;
            }
        }
    }

    pub fn set_current_line(&mut self, line: RebaseTodoLine) {
        self.todo.lines_mut()[self.line] = line;
    }

    pub fn get_current_line(&self) -> Option<&RebaseTodoLine> {
        self.todo.lines().get(self.line)
    }

    pub fn save(&self) -> Result<(), color_eyre::Report> {
        let content = self
            .todo
            .lines()
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&self.path, content)?;
        Ok(())
    }
}

impl Editor for RebaseEditor {
    fn render(&mut self, frame: &mut ratatui::Frame) {
        let areas =
            Layout::horizontal([Constraint::Max(36), Constraint::Fill(1)]).split(frame.area());

        // Render list
        let area = Layout::vertical(
            self.todo
                .lines()
                .iter()
                .map(|_| Constraint::Length(1))
                .collect::<Vec<_>>(),
        )
        .split(areas[0]);

        let lines = self.todo.lines().iter().enumerate().map(|(i, line)| {
            let style = if i == self.line {
                line.get_selected_style()
            } else {
                line.get_style()
            };

            Paragraph::new(line.to_line()).style(style)
        });

        for (i, line) in lines.enumerate() {
            frame.render_widget(line, area[i]);
        }
    }

    fn run(&mut self, mut terminal: ratatui::DefaultTerminal) -> color_eyre::Result<()> {
        terminal.clear()?;
        loop {
            terminal.draw(|frame| self.render(frame))?;
            let line = self.get_current_line();
            let commit = line.and_then(|l| l.get_commit());

            match (event::read()?, commit) {
                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Down,
                        ..
                    }),
                    _,
                ) => self.move_cursor_down(),
                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Up, ..
                    }),
                    _,
                ) => self.move_cursor_up(),

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('p'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    self.set_current_line(RebaseTodoLine::Pick {
                        commit: commit.to_string(),
                        rest: vec![],
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('e'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    self.set_current_line(RebaseTodoLine::Edit {
                        commit: commit.to_string(),
                        rest: vec![],
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('s'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    self.set_current_line(RebaseTodoLine::Squash {
                        commit: commit.to_string(),
                        rest: vec![],
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('f'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    self.set_current_line(RebaseTodoLine::Fixup {
                        commit: commit.to_string(),
                        rest: vec![],
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    self.set_current_line(RebaseTodoLine::Drop {
                        commit: commit.to_string(),
                        rest: vec![],
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('q'),
                        ..
                    }),
                    _,
                ) => {
                    self.save()?;
                    terminal.clear()?;
                    return Ok(());
                }
                _ => {}
            };
        }
    }
}
