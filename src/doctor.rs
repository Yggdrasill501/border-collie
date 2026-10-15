//! `border-collie doctor`: a health report to read or paste into a bug report.

use serde_json::json;

use crate::config::{self, Config};
use crate::herdr::{self, Result};
use crate::state::Store;

fn on_off(enabled: bool) -> &'static str {
    if enabled {
        "on"
    } else {
        "off"
    }
}

fn print_config(config: &Config) {
    println!("  sort_spaces    {}", on_off(config.sort_spaces));
    println!("  sort_by        {}", config.sort_by.name());
    println!("  name_tabs      {}", on_off(config.name_tabs));
    println!("  name_agents    {}", on_off(config.name_agents));
    println!("  dwell_ms       {}", config.dwell_ms);
    println!("  tab_label_max  {}", config.tab_label_max);
}

/// Prints the report. Fails only when herdr cannot be reached, which is the
/// one problem that stops everything else from working.
pub fn run() -> Result<()> {
    println!("border-collie {}", env!("CARGO_PKG_VERSION"));

    let herdr = herdr::request("ping", json!({}));
    match &herdr {
        Ok(pong) => println!(
            "herdr {}: reachable",
            pong["version"].as_str().unwrap_or("?")
        ),
        Err(error) => println!("herdr: {error}"),
    }

    let path = config::config_file();
    let path = path
        .as_deref()
        .map_or("unknown".into(), |p| p.display().to_string());
    match config::read() {
        Some((config, warnings)) => {
            println!("config: {path}");
            print_config(&config);
            for warning in warnings {
                println!("  warning: {warning}");
            }
        }
        None => {
            println!("config: none at {path}, using defaults");
            print_config(&Config::default());
        }
    }

    let store = Store::open()?;
    let state = store.load();
    let dir = store
        .dir()
        .map_or("unknown".into(), |p| p.display().to_string());
    println!("state: {dir}");
    println!("  tabs named     {}", state.tabs.len());
    println!("  agents named   {}", state.agents.len());
    println!("  pinned spaces  {}", state.pins.len());

    herdr.map(|_| ())
}
