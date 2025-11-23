use git2::Repository;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::ToLine,
    widgets::{Block, Borders, Paragraph},
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
    #[allow(dead_code)]
    repo: Repository,
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

        let repo = Repository::discover(
            path.parent()
                .ok_or_else(|| color_eyre::eyre::eyre!("Invalid path"))?,
        )?;

        Ok(Self {
            path,
            todo,
            line,
            repo,
        })
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

    pub fn save_empty(&self) -> Result<(), color_eyre::Report> {
        std::fs::write(&self.path, "")?;
        Ok(())
    }

    pub fn render_todo_list(&self, frame: &mut ratatui::Frame, area: Rect) {
        let block = Block::default().title("Todo").borders(Borders::ALL);

        let lines: Vec<_> = self
            .todo
            .lines()
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let style = if i == self.line {
                    line.get_selected_style()
                } else {
                    line.get_style()
                };

                line.to_line().style(style)
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    pub fn render_commit_info(&self, frame: &mut ratatui::Frame, area: Rect) {
        let line = self.get_current_line();
        let commit = line.and_then(|l| l.get_commit());

        let commit = match commit {
            Some(commit) => commit,
            None => return,
        };

        let block = Block::default().title("Commit").borders(Borders::ALL);

        let info = format!("Hash: {}", commit);

        let paragraph = Paragraph::new(info)
            .block(block)
            .style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(paragraph, area);
    }

    pub fn render_instructions(&self, frame: &mut ratatui::Frame, area: Rect) {
        let instructions = Paragraph::new(
            "↑/↓: Move  p: pick  e: edit  s: squash  f: fixup  d: drop  q: quit and save  a: abort",
        )
        .style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(instructions, area);
    }
}

impl Editor for RebaseEditor {
    fn render(&mut self, frame: &mut ratatui::Frame) {
        let main_area =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(frame.area());

        self.render_instructions(frame, main_area[0]);

        let editor_area =
            Layout::horizontal([Constraint::Max(36), Constraint::Fill(1)]).split(main_area[1]);

        self.render_todo_list(frame, editor_area[0]);
        self.render_commit_info(frame, editor_area[1]);
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

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('a'),
                        ..
                    }),
                    _,
                ) => {
                    terminal.clear()?;
                    self.save_empty()?;
                    return Ok(());
                }

                _ => {}
            };
        }
    }
}
