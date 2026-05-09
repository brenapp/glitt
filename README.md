# Glitt

`glitt` is a small editor for standard git workflows that makes it easier to use Git from the terminal. Instead of being a git TUI first, glitt aims to augment your existing CLI usage by providing dedicated editors for specific git operations to make them easier to complete from your terminal.


## Supported Editors

- Rebase Editor

## Install

You can install `glitt` using cargo.

```
cargo install glitt
```

## Configure

Configure `glitt` as your `core.editor` in your user, local, or system settings.

```
git config --global core.editor "glitt --fallback vim $@"

```
