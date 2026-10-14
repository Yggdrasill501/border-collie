//! border-collie: herdr plugin commands, run by the hooks in `herdr-plugin.toml`.

mod agents;
mod attention;
mod config;
mod doctor;
mod herdr;
mod label;
mod picker;
mod reset;
mod spaces;
mod state;
mod tabs;

use std::env;
use std::process::ExitCode;
use std::time::Duration;

use config::Config;

const USAGE: &str = "usage: border-collie <command>

commands:
  focused      move the focused space up once it has stayed focused
  name         name tabs and agents after what the agents are working on
  startup      do both immediately
  pin          pin the current space to a top slot, or unpin it
  pins         list pinned spaces
  pick         choose an agent to jump to, in this terminal
  open-picker  open the agent picker in a herdr popup
  next         jump to the next agent that needs attention
  reset        restore tab numbers, clear agent names and forget pins
  doctor       report versions, settings and saved state
  --version    print the version";

fn sort_spaces(config: &Config, dwell: Duration) -> herdr::Result<()> {
    if config.sort_spaces {
        spaces::promote_focused(dwell)?;
    }
    Ok(())
}

fn name(config: &Config) -> herdr::Result<()> {
    if !(config.name_tabs || config.name_agents) {
        return Ok(());
    }
    let store = state::Store::open()?;
    let mut state = store.load();
    let before = state.clone();
    let agents = herdr::list("agent.list", "agents")?;

    let tabs = if config.name_tabs {
        tabs::rename_all(&agents, config.tab_label_max, &mut state.tabs)
    } else {
        Ok(())
    };
    let agents = if config.name_agents {
        agents::rename_all(&agents, &mut state.agents)
    } else {
        Ok(())
    };
    if state != before {
        store.save(&state)?;
    }
    tabs.and(agents)
}

fn run(command: &str) -> herdr::Result<()> {
    match command {
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        "--version" | "-V" => {
            println!("border-collie {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "focused" => {
            let config = config::load();
            sort_spaces(&config, Duration::from_millis(config.dwell_ms))
        }
        "name" => name(&config::load()),
        "pin" => spaces::toggle_pin(),
        "pins" => spaces::list_pins(),
        "next" => attention::jump(),
        "pick" => picker::run(),
        "reset" => reset::run(),
        "doctor" => doctor::run(),
        "open-picker" => picker::open(),
        "startup" => {
            let config = config::load();
            let sorted = sort_spaces(&config, Duration::ZERO);
            name(&config).and(sorted)
        }
        "" => Err(USAGE.into()),
        other => Err(format!("unknown command `{other}`\n{USAGE}").into()),
    }
}

fn main() -> ExitCode {
    let command = env::args().nth(1).unwrap_or_default();
    match run(&command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("border-collie: {error}");
            ExitCode::FAILURE
        }
    }
}
