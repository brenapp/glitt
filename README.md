# Glitt

`glitt` is a small editor for standard git workflows that makes it easier to use Git from the terminal. We currently provide an interactive rebase editor. When you configure glitt as your git editor, you can configure a fallback.

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
