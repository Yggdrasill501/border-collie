//! collie: herdr plugin commands, run by the hooks in `herdr-plugin.toml`.

mod herdr;
mod spaces;
mod tabs;

use std::env;
use std::process::ExitCode;
use std::time::Duration;

const USAGE: &str = "usage: collie <startup|focused|name>

  focused  move the focused space to the top once it has stayed focused
  name     rename tabs after their most recently active agent
  startup  do both immediately";

/// How long a space must stay focused before it is promoted.
fn dwell() -> Duration {
    let millis = env::var("COLLIE_DWELL_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1000);
    Duration::from_millis(millis)
}

fn run(command: &str) -> herdr::Result<()> {
    match command {
        "focused" => spaces::promote_focused(dwell()),
        "name" => tabs::rename_all(),
        "startup" => {
            spaces::promote_focused(Duration::ZERO)?;
            tabs::rename_all()
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let command = env::args().nth(1).unwrap_or_default();
    match run(&command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("collie: {error}");
            ExitCode::FAILURE
        }
    }
}
