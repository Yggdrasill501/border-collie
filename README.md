# collie

A [herdr](https://herdr.dev) plugin that keeps your spaces and tabs in order.

- **Recent spaces on top.** When you switch to a space and stay there for a
  second, it moves to the top of the list. Space 1 is always the current one
  and space 2 the one you were in before.
- **Tabs named after their agents.** A tab takes the terminal title of its most
  recently active agent, or the working directory name when the title is
  generic. Tabs you rename yourself are left alone.

## Install

Requires `cargo`. Linux and macOS only.

```bash
herdr plugin install Yggdrasill501/collie
```

For local development, build and link the working tree instead:

```bash
cargo build --release
herdr plugin link "$PWD"
```

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `COLLIE_DWELL_MS` | `1000` | How long a space must stay focused before it moves to the top. |

## How it works

herdr runs `collie` from the event hooks in `herdr-plugin.toml`; there is no
background process. Each run talks to herdr over its socket API and exits.

| Command | Hook | Effect |
| --- | --- | --- |
| `collie focused` | `workspace.focused` | Moves the focused space to the top. |
| `collie name` | agent and pane events | Renames tabs after their agents. |
| `collie startup` | herdr startup | Does both immediately. |
