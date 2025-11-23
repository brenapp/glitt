use std::fmt::Display;

use clap::Parser;
use ratatui::style::{Color, Modifier, Style};

#[derive(clap::Subcommand, Debug)]
pub enum RebaseTodoLine {
    #[command(skip)]
    Comment { message: String },

    #[command(alias = "p")]
    Pick {
        commit: String,
        #[arg(num_args = 1.., trailing_var_arg = true)]
        rest: Vec<String>,
    },

    #[command(alias = "e")]
    Edit {
        commit: String,
        #[arg(num_args = 1.., trailing_var_arg = true)]
        rest: Vec<String>,
    },

    #[command(alias = "s")]
    Squash {
        commit: String,
        #[arg(num_args = 1.., trailing_var_arg = true)]
        rest: Vec<String>,
    },

    #[command(alias = "f")]
    Fixup {
        commit: String,
        #[arg(num_args = 1.., trailing_var_arg = true)]
        rest: Vec<String>,
    },

    #[command(alias = "x")]
    Exec {
        #[arg(num_args = 1.., trailing_var_arg = true)]
        command: Vec<String>,
    },

    #[command(alias = "d")]
    Drop {
        commit: String,
        #[arg(num_args = 1.., trailing_var_arg = true)]
        rest: Vec<String>,
    },

    #[command(alias = "l")]
    Label {
        label: String,
        #[arg(num_args = 1.., trailing_var_arg = true)]
        rest: Vec<String>,
    },

    #[command(alias = "r")]
    Reset {
        label: String,
        #[arg(num_args = 1.., trailing_var_arg = true)]
        rest: Vec<String>,
    },

    #[command(alias = "m")]
    Merge {
        #[arg(short = 'c', alias = "C")]
        commit: Option<String>,
        label: String,
    },

    #[command(alias = "u")]
    UpdateRef { refname: String },
}

#[derive(Parser, Debug)]
#[command(no_binary_name = true)]
struct RebaseTodoLineParser {
    #[command(subcommand)]
    line: RebaseTodoLine,
}

impl RebaseTodoLine {
    pub fn get_color(&self) -> Color {
        match self {
            RebaseTodoLine::Comment { .. } => Color::White,
            RebaseTodoLine::Pick { .. } => Color::White,
            RebaseTodoLine::Edit { .. } => Color::Blue,
            RebaseTodoLine::Squash { .. } => Color::Yellow,
            RebaseTodoLine::Fixup { .. } => Color::LightYellow,
            RebaseTodoLine::Exec { .. } => Color::Red,
            RebaseTodoLine::Drop { .. } => Color::White,
            RebaseTodoLine::Label { .. } => Color::White,
            RebaseTodoLine::Reset { .. } => Color::White,
            RebaseTodoLine::Merge { .. } => Color::White,
            RebaseTodoLine::UpdateRef { .. } => Color::White,
        }
    }

    pub fn get_style(&self) -> Style {
        let color = self.get_color();
        match self {
            RebaseTodoLine::Comment { .. } => {
                Style::default().fg(color).add_modifier(Modifier::DIM)
            }
            RebaseTodoLine::Drop { .. } => Style::default()
                .fg(color)
                .add_modifier(Modifier::CROSSED_OUT)
                .add_modifier(Modifier::DIM),
            _ => Style::default().fg(color),
        }
    }
}

impl Display for RebaseTodoLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RebaseTodoLine::Comment { message } => write!(f, "{}", message),
            RebaseTodoLine::Pick { commit, .. } => write!(f, "pick {}", commit),
            RebaseTodoLine::Edit { commit, .. } => write!(f, "edit {}", commit),
            RebaseTodoLine::Squash { commit, .. } => write!(f, "squash {}", commit),
            RebaseTodoLine::Fixup { commit, .. } => write!(f, "fixup {}", commit),
            RebaseTodoLine::Exec { command, .. } => write!(f, "exec {}", command.join(" ")),
            RebaseTodoLine::Drop { commit, .. } => write!(f, "drop {}", commit),
            RebaseTodoLine::Label { label, .. } => write!(f, "label {}", label),
            RebaseTodoLine::Reset { label, .. } => write!(f, "reset {}", label),
            RebaseTodoLine::Merge { commit, label } => {
                if let Some(c) = commit {
                    write!(f, "merge -c {} {}", c, label)
                } else {
                    write!(f, "merge {}", label)
                }
            }
            RebaseTodoLine::UpdateRef { refname } => write!(f, "update-ref {}", refname),
        }
    }
}

impl RebaseTodoLine {
    pub fn parse(line: &str) -> Self {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            RebaseTodoLine::Comment {
                message: line.to_string(),
            }
        } else {
            RebaseTodoLineParser::try_parse_from(line.split_whitespace())
                .map(|parser| parser.line)
                .unwrap_or(RebaseTodoLine::Comment {
                    message: line.to_string(),
                })
        }
    }
}

pub struct RebaseTodo {
    lines: Vec<RebaseTodoLine>,
}

impl RebaseTodo {
    pub fn parse(content: &str) -> Self {
        let lines = content
            .lines()
            .map(RebaseTodoLine::parse)
            .collect::<Vec<_>>();
        RebaseTodo { lines }
    }

    pub fn lines(&self) -> &Vec<RebaseTodoLine> {
        &self.lines
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_comment_hash() {
        let line = RebaseTodoLine::parse("# this is a comment");
        assert_eq!(format!("{}", line), "# this is a comment");
    }

    #[test]
    fn parse_empty_line_is_comment() {
        let line = RebaseTodoLine::parse("");
        assert_eq!(format!("{}", line), "");
    }

    #[test]
    fn parse_pick_and_alias() {
        let pick = RebaseTodoLine::parse("pick abc123");
        assert_eq!(format!("{}", pick), "pick abc123");

        let alias = RebaseTodoLine::parse("p abc123");
        // alias should parse to the canonical "pick" form when displayed
        assert_eq!(format!("{}", alias), "pick abc123");
    }

    #[test]
    fn parse_edit_squash_fixup_drop_label_reset_update_ref() {
        let cases = vec![
            ("edit deadbeef", "edit deadbeef"),
            ("e deadbeef", "edit deadbeef"),
            ("s deadbeef", "squash deadbeef"),
            ("f deadbeef", "fixup deadbeef"),
            ("d deadbeef", "drop deadbeef"),
            ("l mylabel", "label mylabel"),
            ("r mylabel", "reset mylabel"),
            ("u refs/heads/main", "update-ref refs/heads/main"),
        ];

        for (input, expected) in cases {
            let parsed = RebaseTodoLine::parse(input);
            assert_eq!(format!("{}", parsed), expected, "input: {}", input);
        }
    }

    #[test]
    fn parse_exec_with_multiple_args() {
        let line = RebaseTodoLine::parse("exec echo hello world");
        assert_eq!(format!("{}", line), "exec echo hello world");
        let alias = RebaseTodoLine::parse("x echo hello world");
        assert_eq!(format!("{}", alias), "exec echo hello world");
    }

    #[test]
    fn parse_merge_with_and_without_commit_flag() {
        let without = RebaseTodoLine::parse("merge feature_branch");
        assert_eq!(format!("{}", without), "merge feature_branch");

        let with_c = RebaseTodoLine::parse("merge -c abc123 feature_branch");
        assert_eq!(format!("{}", with_c), "merge -c abc123 feature_branch");

        // alias 'm' should behave like 'merge'
        let alias = RebaseTodoLine::parse("m -c abc123 feature_branch");
        assert_eq!(format!("{}", alias), "merge -c abc123 feature_branch");
    }

    #[test]
    fn parse_rebase_todo_multiple_lines() {
        let content = "# top comment\npick a1b2c3d\ndrop deadbeef\n\nexec echo hi\n";
        let todo = RebaseTodo::parse(content);
        let rendered = todo
            .lines()
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            rendered,
            vec![
                "# top comment",
                "pick a1b2c3d",
                "drop deadbeef",
                "",
                "exec echo hi"
            ]
        );
    }
}
