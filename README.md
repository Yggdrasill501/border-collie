# collie

[![CI](https://github.com/Yggdrasill501/collie/actions/workflows/ci.yml/badge.svg)](https://github.com/Yggdrasill501/collie/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A [herdr](https://herdr.dev) plugin that keeps the herd in order: spaces sorted
by recent use, and tabs and agents named after the work happening in them.

```text
 before                         after
 ──────                         ─────
 spaces   1 ~                   1 meety-clone      ← where you are
          2 meety               2 meety            ← where you were
          3 daily-log           3 ~
          4 meety-clone         4 daily-log

 tabs     1 │ 2 │ 3             PR 457 │ Linear tasks needing at… │ meety

 agents   w9:p1                 pr-457
          w6:p3                 linear-tasks-needing-attention
```

## Features

- **Recent spaces on top.** Switch to a space, stay a second, and it moves to
  the top. Without pins, space 1 is always the current one and space 2 the
  one you came from, so jumping back is always the same key.
- **Tabs named after their agents.** A tab takes the terminal title of its
  most recently active agent, or the directory name when the title is generic.
- **Agents named after their work.** Each agent gets a unique name such as
  `pr-457`, which herdr accepts anywhere it takes an agent:
  `herdr agent focus pr-457`.
- **Pinned spaces.** Pin the spaces you live in to fixed top slots, the way
  [harpoon](https://github.com/ThePrimeagen/harpoon) pins files. Everything
  else stays sorted by recent use below them.
- **Agent picker.** One key opens a list of every agent, most recently active
  first. Press a digit to jump straight there.
- **Your names win.** Tabs and agents you rename yourself are never touched.
- **No background process.** herdr runs a small Rust binary from its event
  hooks; each run finishes in a few milliseconds.

## Requirements

- herdr 0.9 or newer, on Linux or macOS
- Rust 1.89 or newer (`cargo`), to build the plugin on install

## Installation

```bash
herdr plugin install Yggdrasill501/collie
```

That is all: collie starts working on the next space switch or agent update.
To apply it to everything immediately:

```bash
herdr plugin action invoke collie.name
```

To update, run the install command again. To remove it:

```bash
herdr plugin uninstall collie
```

## Configuration

collie works without configuration. To change a setting, create `config.toml`
in the plugin's config directory, which `herdr plugin config-dir collie`
prints:

```toml
sort_spaces   = true   # move the focused space to the top
name_tabs     = true   # name tabs after their agents
name_agents   = true   # name agents after their work
dwell_ms      = 1000   # how long a space must stay focused before it moves
tab_label_max = 24     # longest tab label, in characters
```

Every line is optional; the values above are the defaults. Changes apply on
the next hook run, with no reload.

### Suggested keybindings

Sorted spaces pair well with number keys. In herdr's own `config.toml`:

```toml
[keys]
switch_workspace = "prefix+1..9"        # prefix+2 is always the previous space
switch_tab       = "prefix+shift+1..9"
```

Pin or unpin the current space (see [Pinned spaces](#pinned-spaces)):

```toml
[[keys.command]]
key = "prefix+shift+p"
type = "plugin_action"
command = "collie.toggle-pin"
description = "pin or unpin this space"
```

Open the [agent picker](#agent-picker):

```toml
[[keys.command]]
key = "prefix+a"
type = "plugin_action"
command = "collie.pick-agent"
description = "jump to an agent"
```

To rename everything on demand:

```toml
[[keys.command]]
key = "prefix+shift+c"
type = "plugin_action"
command = "collie.name"
description = "name tabs and agents"
```

## Pinned spaces

Recent-first ordering means a space's number changes as you move around.
For the few spaces you want on a fixed key, pin them:

```bash
herdr plugin action invoke collie.toggle-pin   # pin or unpin the current space
```

Pinned spaces take the top slots in the order you pinned them, and stay
there. With two pins, `prefix+1` and `prefix+2` always open those two spaces,
`prefix+3` is the most recent of the rest, and so on.

```text
 1 meety             pinned
 2 meety-clone       pinned
 3 daily-log         most recent of the rest
 4 ~
```

Run the action again on a pinned space to unpin it. Pins are forgotten when
their space is closed.

## Agent picker

The picker opens in a popup and lists agents from every space, most recently
active first, with their status and space:

```text
 Jump to agent
 1  ● working  meety-clone        PR 457
 2  ! blocked  meety              Linear tasks needing attention
 3  ✓ done     daily-log          GitHub contributions backfill
 4  · idle     ~                  Slack MCP for multiple accounts
 1-9 jump · j/k move · enter select · esc cancel
```

| Key | Action |
| --- | --- |
| `1`-`9` | Jump to that agent |
| `j` / `k`, arrow keys | Move the selection |
| `enter` | Jump to the selected agent |
| `esc`, `q` | Close |

Bind it to a key as shown in [Suggested keybindings](#suggested-keybindings),
or open it with `herdr plugin action invoke collie.pick-agent`.

## How names are chosen

| | Source | Example |
| --- | --- | --- |
| Tab | Title of the tab's most recently active agent, cut to `tab_label_max` | `Linear tasks needing at…` |
| Agent | The same title in herdr's name alphabet (`a-z`, `0-9`, `-`), at most 32 characters | `linear-tasks-needing-attention` |

When a title is empty or generic (`Claude Code`), the directory name is used
instead. Two agents that would get the same name become `meety` and `meety-2`.

collie only renames a tab that still has herdr's numeric label, or an agent
that has no name, or one whose current name collie set. Rename something
yourself and collie leaves it alone from then on. To hand a tab back, rename
it to a number.

## How it works

herdr runs `collie` from the hooks in [`herdr-plugin.toml`](herdr-plugin.toml).
Each run talks to herdr over its socket API and exits.

| Command | Runs on | Effect |
| --- | --- | --- |
| `collie focused` | `workspace.focused` | Moves the focused space above the other unpinned spaces if it is still focused after `dwell_ms`. |
| `collie name` | agent and pane events | Names tabs and agents. |
| `collie startup` | herdr startup | Does both immediately. |
| `collie pin` | the `toggle-pin` action | Pins or unpins the current space. |
| `collie pick` | the `pick-agent` action, in a popup | Shows the agent picker. |

## Troubleshooting

See what collie did, and any errors:

```bash
herdr plugin log list --plugin collie
```

- **Nothing happens after install.** Check that `herdr plugin list` shows
  collie as enabled, then run `herdr plugin action invoke collie.name`.
- **A space jumps to the top too eagerly, or too late.** Adjust `dwell_ms`.
- **A tab or agent is not renamed.** It has a name collie did not set. See
  [How names are chosen](#how-names-are-chosen).

## Contributing

Issues and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
