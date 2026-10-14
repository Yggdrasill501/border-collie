# Changelog

All notable changes to collie are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/Yggdrasill501/collie/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Yggdrasill501/collie/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Yggdrasill501/collie/releases/tag/v0.1.0
