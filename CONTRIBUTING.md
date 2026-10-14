# Contributing

Bug reports, ideas and pull requests are welcome.

## Development

You need Rust 1.89 or newer and a running [herdr](https://herdr.dev) 0.9+.

```bash
cargo build --release
herdr plugin link "$PWD"      # run this checkout as the installed plugin
```

herdr runs `target/release/collie` from the hooks in `herdr-plugin.toml`, so
rebuild after each change. You can also run the commands by hand:

```bash
target/release/collie name
herdr plugin log list --plugin collie   # output of the hook runs
```

Before opening a pull request:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## Layout

| File | Purpose |
| --- | --- |
| `herdr-plugin.toml` | Plugin manifest: which herdr events run which command. |
| `src/main.rs` | Command line entry point. |
| `src/herdr.rs` | Client for herdr's socket API. |
| `src/spaces.rs` | Recent-first space ordering. |
| `src/tabs.rs`, `src/agents.rs` | Tab and agent naming. |
| `src/picker.rs` | Interactive agent picker. |
| `src/label.rs` | Turning titles into labels and names. |
| `src/config.rs`, `src/state.rs` | Settings and remembered names. |

Decision logic is kept in pure functions with unit tests next to the code;
please add a test with any behaviour change.
