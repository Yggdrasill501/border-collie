# Changelog

All notable changes to border-collie are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.5.0] - 2026-10-14

### Added

- `border-collie.next-attention` action: jumps to the next agent that is
  blocked or has finished, blocked agents first.
- `border-collie.reset` action: restores tab numbers, clears agent names and
  forgets pins, for a clean uninstall.
- `border-collie.doctor` action: reports versions, settings and saved state.

### Changed

- The agent picker starts on the first agent other than the focused one, so
  opening it and pressing enter switches to the previous agent.

- Names are also refreshed when a pane's process exits.

### Fixed

- Commands run by hand now read `config.toml` instead of using defaults.

## [0.4.0] - 2026-10-14

The plugin is now called **border-collie**, to tell it apart from an unrelated
herdr project named collie. Earlier entries below use the old name.

### Upgrading from collie

- Uninstall the old plugin and install the new one:
  `herdr plugin uninstall collie`, then
  `herdr plugin install Yggdrasill501/border-collie`.
- Rename actions in your keybindings: `collie.pick-agent` becomes
  `border-collie.pick-agent`, and likewise for `toggle-pin` and `name`.
- Move `config.toml`, if you have one, to the directory printed by
  `herdr plugin config-dir border-collie`.
- Pins and remembered names carry over on their own.

### Changed

- A tab that collie named goes back to its number when its last agent exits,
  instead of keeping a label for work that is gone.
- Titles such as `Gemini CLI` are treated as generic, like `Claude Code`, and
  replaced by the directory name.

### Fixed

- Agent picker: keys that send escape sequences containing digits, such as
  ctrl+arrow, no longer jump to an agent.
- A new agent name can no longer collide with the name of an agent collie
  could not describe.
- Errors from herdr are reported as readable messages, and a missing socket
  names the path that was tried.

## [0.3.0] - 2026-10-14

### Added

- Pinned spaces: the `collie.toggle-pin` action pins the current space to a
  fixed top slot. Unpinned spaces stay sorted by recent use below the pins.
- Agent picker: the `collie.pick-agent` action opens a popup listing every
  agent, most recently active first. Press a digit to jump to one.

## [0.2.0] - 2026-10-14

### Added

- Agents are named after what they are working on, for example `pr-457`, so
  they can be addressed by name: `herdr agent focus pr-457`.
- `config.toml` in the plugin config directory, with switches for each feature,
  the focus delay and the tab label length.
- `collie --version`.

### Changed

- The focus delay is set with `dwell_ms` in `config.toml`; the
  `COLLIE_DWELL_MS` environment variable is no longer read.

### Fixed

- Hooks that fire at the same time no longer race: naming runs take a lock and
  the state file is replaced atomically.
- A tab whose rename fails no longer stops the remaining tabs from being named.

## [0.1.0] - 2026-10-13

### Added

- Spaces are kept sorted by most recently focused.
- Tabs are named after their most recently active agent.

[Unreleased]: https://github.com/Yggdrasill501/border-collie/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/Yggdrasill501/border-collie/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Yggdrasill501/border-collie/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Yggdrasill501/border-collie/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Yggdrasill501/border-collie/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Yggdrasill501/border-collie/releases/tag/v0.1.0
