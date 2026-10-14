//! User settings, read from `config.toml` in the plugin's config directory.

use std::env;
use std::fs;
use std::path::PathBuf;

const SHORTEST_TAB_LABEL: usize = 4;

#[derive(Debug, PartialEq)]
pub struct Config {
    /// Move the focused space to the top of the list.
    pub sort_spaces: bool,
    /// Rename tabs after their most recently active agent.
    pub name_tabs: bool,
    /// Give agents a name based on what they are working on.
    pub name_agents: bool,
    /// How long a space must stay focused before it is promoted.
    pub dwell_ms: u64,
    /// Longest tab label border-collie will set, in characters.
    pub tab_label_max: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sort_spaces: true,
            name_tabs: true,
            name_agents: true,
            dwell_ms: 1000,
            tab_label_max: 24,
        }
    }
}

fn config_file() -> Option<PathBuf> {
    env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(|dir| PathBuf::from(dir).join("config.toml"))
}

/// Loads the config, falling back to defaults for anything missing or
/// invalid. Problems are reported on stderr, which herdr keeps in the plugin
/// log.
pub fn load() -> Config {
    let Some(text) = config_file().and_then(|path| fs::read_to_string(path).ok()) else {
        return Config::default();
    };
    let (config, warnings) = parse(&text);
    for warning in warnings {
        eprintln!("border-collie: config.toml: {warning}");
    }
    config
}

/// Parses flat `key = value` lines. This is the subset of TOML the config
/// needs: booleans, integers and `#` comments.
fn parse(text: &str) -> (Config, Vec<String>) {
    let mut config = Config::default();
    let mut warnings = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            warnings.push(format!("line {}: expected `key = value`", number + 1));
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        let applied = match key {
            "sort_spaces" => value.parse().map(|v| config.sort_spaces = v).is_ok(),
            "name_tabs" => value.parse().map(|v| config.name_tabs = v).is_ok(),
            "name_agents" => value.parse().map(|v| config.name_agents = v).is_ok(),
            "dwell_ms" => value.parse().map(|v| config.dwell_ms = v).is_ok(),
            "tab_label_max" => value.parse().map(|v| config.tab_label_max = v).is_ok(),
            _ => {
                warnings.push(format!("line {}: unknown setting `{key}`", number + 1));
                continue;
            }
        };
        if !applied {
            warnings.push(format!(
                "line {}: invalid value `{value}` for `{key}`",
                number + 1
            ));
        }
    }
    if config.tab_label_max < SHORTEST_TAB_LABEL {
        warnings.push(format!(
            "tab_label_max raised to the minimum of {SHORTEST_TAB_LABEL}"
        ));
        config.tab_label_max = SHORTEST_TAB_LABEL;
    }
    (config, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_is_the_default() {
        assert_eq!(parse(""), (Config::default(), vec![]));
    }

    #[test]
    fn reads_every_setting() {
        let text = "
            # turn naming off
            sort_spaces = false
            name_tabs = false   # trailing comment
            name_agents = false
            dwell_ms = 250
            tab_label_max = 40
        ";
        let (config, warnings) = parse(text);
        assert!(warnings.is_empty());
        assert_eq!(
            config,
            Config {
                sort_spaces: false,
                name_tabs: false,
                name_agents: false,
                dwell_ms: 250,
                tab_label_max: 40,
            }
        );
    }

    #[test]
    fn invalid_lines_warn_and_keep_defaults() {
        let (config, warnings) = parse("dwell_ms = soon\nspeed = 3\nnonsense\n");
        assert_eq!(config, Config::default());
        assert_eq!(warnings.len(), 3);
        assert!(warnings[0].contains("line 1") && warnings[0].contains("dwell_ms"));
        assert!(warnings[1].contains("unknown setting `speed`"));
        assert!(warnings[2].contains("line 3"));
    }

    #[test]
    fn tab_label_max_has_a_floor() {
        let (config, warnings) = parse("tab_label_max = 0");
        assert_eq!(config.tab_label_max, SHORTEST_TAB_LABEL);
        assert_eq!(warnings.len(), 1);
    }
}
