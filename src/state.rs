//! What border-collie has named, kept between runs in the plugin's state directory.
//!
//! A tab or agent is only renamed while it still carries herdr's default or
//! the name border-collie last gave it, so names set by hand are never overwritten.

use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::herdr::Result;

const LEGACY_PLUGIN_ID: &str = "collie";

pub type Names = HashMap<String, String>;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct State {
    /// Labels border-collie set, keyed by tab id.
    pub tabs: Names,
    /// Names border-collie set, keyed by the agent's terminal id.
    pub agents: Names,
    /// Ids of pinned spaces, in slot order.
    pub pins: Vec<String>,
}

fn state_dir() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("HERDR_PLUGIN_STATE_DIR") {
        return Some(PathBuf::from(dir));
    }
    // Same location herdr uses, for runs outside a plugin hook.
    let base = match env::var_os("XDG_STATE_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env::var_os("HOME")?).join(".local/state"),
    };
    Some(base.join("herdr/plugins/border-collie"))
}

fn names(value: &Value) -> Names {
    let Some(map) = value.as_object() else {
        return Names::new();
    };
    map.iter()
        .filter_map(|(key, name)| Some((key.clone(), name.as_str()?.to_string())))
        .collect()
}

impl State {
    fn from_json(text: &str) -> Self {
        let value: Value = serde_json::from_str(text).unwrap_or_default();
        let pins = value["pins"].as_array().into_iter().flatten();
        Self {
            tabs: names(&value["tabs"]),
            agents: names(&value["agents"]),
            pins: pins
                .filter_map(|id| Some(id.as_str()?.to_string()))
                .collect(),
        }
    }

    fn to_json(&self) -> String {
        json!({ "tabs": self.tabs, "agents": self.agents, "pins": self.pins }).to_string()
    }
}

/// Exclusive access to the state for the lifetime of the value, so hooks that
/// fire together rename one after another instead of racing.
pub struct Store {
    dir: Option<PathBuf>,
    _lock: Option<File>,
}

impl Store {
    pub fn open() -> Result<Self> {
        let Some(dir) = state_dir() else {
            return Ok(Self {
                dir: None,
                _lock: None,
            });
        };
        fs::create_dir_all(&dir)?;
        let lock = File::create(dir.join("lock"))?;
        lock.lock()?;
        Ok(Self {
            dir: Some(dir),
            _lock: Some(lock),
        })
    }

    pub fn load(&self) -> State {
        let Some(dir) = &self.dir else {
            return State::default();
        };
        if let Ok(text) = fs::read_to_string(dir.join("state.json")) {
            return State::from_json(&text);
        }
        // Before 0.4 the plugin was called collie and kept its state under
        // that name.
        let legacy = dir.with_file_name(LEGACY_PLUGIN_ID).join("state.json");
        let Ok(text) = fs::read_to_string(legacy) else {
            return State::default();
        };
        let state = State::from_json(&text);
        // Copy it over so the old directory can be removed.
        let _ = self.save(&state);
        state
    }

    /// Replaces the state file in one step so a crash cannot leave it torn.
    pub fn save(&self, state: &State) -> Result<()> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let temporary = dir.join("state.json.tmp");
        fs::write(&temporary, state.to_json())?;
        fs::rename(temporary, dir.join("state.json"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips_through_json() {
        let mut state = State::default();
        state.tabs.insert("w1:t1".into(), "PR 457".into());
        state.agents.insert("term_1".into(), "pr-457".into());
        state.pins = vec!["w3".into(), "w1".into()];
        assert_eq!(State::from_json(&state.to_json()), state);
    }

    #[test]
    fn unreadable_state_is_empty() {
        assert_eq!(State::from_json("not json"), State::default());
        assert_eq!(State::from_json(r#"{"tabs": [1, 2]}"#), State::default());
    }
}
