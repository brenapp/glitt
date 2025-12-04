use crate::editors::{
    Editor,
    rebase::todo::{RebaseTodo, RebaseTodoLine},
};
use git2::{Commit, Repository};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{
    path::{Path, PathBuf},
    str,
};

pub struct RebaseEditor {
    path: PathBuf,
    todo: RebaseTodo,
    repo: Repository,
    list_state: ListState,
}

impl RebaseEditor {
    pub fn new(path: PathBuf) -> Result<Self, color_eyre::Report> {
        let content = std::fs::read_to_string(&path)?;
        let todo = RebaseTodo::parse(&content);

        let initial_line = todo
            .lines()
            .iter()
            .position(|line| !matches!(line, RebaseTodoLine::Comment { .. }))
            .unwrap_or(0);

        let repo = Repository::discover(
            path.parent()
                .ok_or_else(|| color_eyre::eyre::eyre!("Invalid path"))?,
        )?;

        let mut list_state = ListState::default();
        list_state.select(Some(initial_line));

        Ok(Self {
            path,
            todo,
            repo,
            list_state,
        })
    }

    fn selected(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    pub fn move_cursor_down(&mut self) {
        let lines = self.todo.lines();
        let len = lines.len();
        if len == 0 {
            return;
        }

        let mut idx = self.selected();
        for _ in 0..len {
            idx = (idx + 1) % len;
            if !matches!(lines[idx], RebaseTodoLine::Comment { .. }) {
                self.list_state.select(Some(idx));
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

        let mut idx = self.selected();
        for _ in 0..len {
            if idx == 0 {
                idx = len - 1;
            } else {
                idx -= 1;
            }
            if !matches!(lines[idx], RebaseTodoLine::Comment { .. }) {
                self.list_state.select(Some(idx));
                return;
            }
        }
    }

    pub fn swap_down(&mut self) {
        let lines = self.todo.lines();
        let len = lines.len();
        if len == 0 {
            return;
        }

        let current_line = self.selected();
        let mut idx = current_line;
        for _ in 0..len {
            idx = (idx + 1) % len;
            if !matches!(lines[idx], RebaseTodoLine::Comment { .. }) {
                self.todo.lines_mut().swap(current_line, idx);
                self.list_state.select(Some(idx));
                return;
            }
        }
    }

    pub fn swap_up(&mut self) {
        let lines = self.todo.lines();
        let len = lines.len();
        if len == 0 {
            return;
        }

        let current_line = self.selected();
        let mut idx = current_line;
        for _ in 0..len {
            if idx == 0 {
                idx = len - 1;
            } else {
                idx -= 1;
            }
            if !matches!(lines[idx], RebaseTodoLine::Comment { .. }) {
                self.todo.lines_mut().swap(current_line, idx);
                self.list_state.select(Some(idx));
                return;
            }
        }
    }

    pub fn set_current_line(&mut self, line: RebaseTodoLine) {
        let idx = self.selected();
        self.todo.lines_mut()[idx] = line;
    }

    pub fn get_current_line(&self) -> Option<&RebaseTodoLine> {
        self.todo.lines().get(self.selected())
    }

    pub fn get_commit_for_line(&self, line: &RebaseTodoLine) -> Option<Commit<'_>> {
        let sha = line.get_commit()?;

        self.repo
            .revparse_single(sha)
            .ok()
            .and_then(|r| r.into_commit().ok())
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

    pub fn render_todo_list(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let block = Block::default().title("Todo").borders(Borders::ALL);
        let selected = self.selected();

        let items: Vec<ListItem> = self
            .todo
            .lines()
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let style = if i == selected {
                    line.get_selected_style()
                } else {
                    line.get_style()
                };

                ListItem::new(Line::from(line.to_string())).style(style)
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn get_commit_diff(&self, commit: &git2::Commit) -> Option<Vec<Line<'_>>> {
        let tree = commit.tree().ok()?;
        let parent = commit.parent(0).ok()?;
        let parent_tree = parent.tree().ok()?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)
            .ok()?;

        let mut diffs = vec![];
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            let style = match line.origin() {
                '+' => Style::default().fg(ratatui::style::Color::Green),
                '-' => Style::default().fg(ratatui::style::Color::Red),
                _ => Style::default(),
            };
            diffs.push(Line::from(Span::styled(
                str::from_utf8(line.content()).unwrap_or("").to_string(),
                style,
            )));
            true
        })
        .ok()?;

        Some(diffs)
    }

    pub fn format_commit(&self, commit: &git2::Commit) -> Paragraph<'_> {
        let timestamp = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
            .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());

        let diff = self.get_commit_diff(commit).unwrap_or_default();

        let mut content = vec![];
        content.push(
            format!(
                "Author: {} <{}>\n",
                commit.author().name().unwrap_or("Unknown"),
                commit.author().email().unwrap_or("unknown")
            )
            .into(),
        );
        content.push(format!("Date:   {}\n\n", timestamp).into());
        content.push("".into());
        content.push(format!("{}\n\n", commit.message().unwrap_or("No commit message")).into());
        content.push("".into());

        content.extend(diff);

        Paragraph::new(content).style(Style::default())
    }

    pub fn render_commit_info(&self, frame: &mut ratatui::Frame, area: Rect) {
        let line = self.get_current_line();
        let commit = line.and_then(|l| self.get_commit_for_line(l));

        let commit = match commit {
            Some(c) => c,
            None => {
                let block = Block::default().title("Commit").borders(Borders::ALL);
                let paragraph = Paragraph::new("No commit selected").block(block);
                frame.render_widget(paragraph, area);
                return;
            }
        };

        let block = Block::default().title("Commit").borders(Borders::ALL);
        let paragraph = self.format_commit(&commit).block(block);

        frame.render_widget(paragraph, area);
    }

    pub fn render_instructions(&self, frame: &mut ratatui::Frame, area: Rect) {
        let instructions = Paragraph::new(format!(
            "{} Move  {} pick  {} edit  {} reword {} squash  {} fixup  {} drop  {} quit and save  {} abort",
            "↑/↓".bold(),
            "p".bold(),
            "e".bold(),
            "r".bold(),
            "s".bold(),
            "f".bold(),
            "d".bold(),
            "q".bold(),
            "a".bold()
        ))
        .style(Style::default());

        frame.render_widget(instructions, area);
    }
}

impl Editor for RebaseEditor {
    fn should_run(path: &Path) -> bool {
        path.file_stem().is_some_and(|f| f.eq("git-rebase-todo"))
    }

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
                        modifiers: KeyModifiers::SHIFT,
                        ..
                    }),
                    _,
                ) => self.swap_down(),
                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Down,
                        ..
                    }),
                    _,
                ) => self.move_cursor_down(),
                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::SHIFT,
                        ..
                    }),
                    _,
                ) => self.swap_up(),

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
                    let rest = line.and_then(|l| l.get_rest()).unwrap_or_default().to_vec();
                    self.set_current_line(RebaseTodoLine::Pick {
                        commit: commit.to_string(),
                        rest: rest,
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('e'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    let rest = line.and_then(|l| l.get_rest()).unwrap_or_default().to_vec();
                    self.set_current_line(RebaseTodoLine::Edit {
                        commit: commit.to_string(),
                        rest,
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('r'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    let rest = line.and_then(|l| l.get_rest()).unwrap_or_default().to_vec();
                    self.set_current_line(RebaseTodoLine::Reword {
                        commit: commit.to_string(),
                        rest,
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('s'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    let rest = line.and_then(|l| l.get_rest()).unwrap_or_default().to_vec();
                    self.set_current_line(RebaseTodoLine::Squash {
                        commit: commit.to_string(),
                        rest,
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('f'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    let rest = line.and_then(|l| l.get_rest()).unwrap_or_default().to_vec();
                    self.set_current_line(RebaseTodoLine::Fixup {
                        commit: commit.to_string(),
                        rest,
                    });
                }

                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        ..
                    }),
                    Some(commit),
                ) => {
                    let rest = line.and_then(|l| l.get_rest()).unwrap_or_default().to_vec();
                    self.set_current_line(RebaseTodoLine::Drop {
                        commit: commit.to_string(),
                        rest,
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
