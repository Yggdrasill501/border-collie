//! Names tabs after the most recently active agent they contain.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::herdr::{self, Result};

const MAX_LABEL_CHARS: usize = 24;

/// Labels collie has set, keyed by tab id. A tab is only renamed while its
/// label is herdr's default or one collie chose, so manual names are kept.
type Named = HashMap<String, String>;

fn state_file() -> Option<PathBuf> {
    env::var_os("HERDR_PLUGIN_STATE_DIR").map(|dir| PathBuf::from(dir).join("tab-names.json"))
}

fn load_named() -> Named {
    state_file()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_named(named: &Named) -> Result<()> {
    if let Some(path) = state_file() {
        fs::write(path, serde_json::to_string(named)?)?;
    }
    Ok(())
}

/// Herdr labels unnamed tabs with their number.
fn is_default_label(label: &str) -> bool {
    !label.is_empty() && label.chars().all(|c| c.is_ascii_digit())
}

fn truncate(label: &str) -> String {
    if label.chars().count() <= MAX_LABEL_CHARS {
        return label.to_string();
    }
    let head: String = label.chars().take(MAX_LABEL_CHARS - 1).collect();
    format!("{}…", head.trim_end())
}

/// The label an agent suggests for its tab: its terminal title, or the
/// working directory name when the title says nothing specific.
fn label_for(agent: &Value) -> Option<String> {
    let title = agent["terminal_title_stripped"]
        .as_str()
        .unwrap_or("")
        .trim();
    let name = agent["agent"].as_str().unwrap_or("");
    let generic = title.is_empty()
        || title.eq_ignore_ascii_case(name)
        || title.eq_ignore_ascii_case(&format!("{name} code"));
    let label = if generic {
        let cwd = agent["cwd"].as_str()?.trim_end_matches('/');
        cwd.rsplit('/').next().filter(|dir| !dir.is_empty())?
    } else {
        title
    };
    Some(truncate(label))
}

/// Picks one label per tab from its most recently active agent.
fn wanted_labels(agents: &[Value]) -> HashMap<&str, String> {
    let mut latest: HashMap<&str, &Value> = HashMap::new();
    for agent in agents {
        let Some(tab) = agent["tab_id"].as_str() else {
            continue;
        };
        let seq = |agent: &Value| agent["state_change_seq"].as_u64().unwrap_or(0);
        let current = latest.entry(tab).or_insert(agent);
        if seq(agent) > seq(current) {
            *current = agent;
        }
    }
    latest
        .into_iter()
        .filter_map(|(tab, agent)| Some((tab, label_for(agent)?)))
        .collect()
}

pub fn rename_all() -> Result<()> {
    let tabs = herdr::list("tab.list", "tabs")?;
    let agents = herdr::list("agent.list", "agents")?;
    let wanted = wanted_labels(&agents);
    let mut named = load_named();
    let before = named.clone();

    named.retain(|id, _| tabs.iter().any(|tab| tab["tab_id"] == id.as_str()));
    for tab in &tabs {
        let (Some(id), Some(label)) = (tab["tab_id"].as_str(), tab["label"].as_str()) else {
            continue;
        };
        let Some(new) = wanted.get(id) else {
            continue;
        };
        let ours = named.get(id).is_some_and(|set| set == label);
        if !(ours || is_default_label(label)) {
            named.remove(id);
            continue;
        }
        if new != label {
            herdr::request("tab.rename", json!({ "tab_id": id, "label": new }))?;
            println!("renamed {id}: {label} -> {new}");
        }
        named.insert(id.to_string(), new.clone());
    }
    if named != before {
        save_named(&named)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(tab: &str, seq: u64, title: &str, cwd: &str) -> Value {
        json!({
            "agent": "claude",
            "tab_id": tab,
            "state_change_seq": seq,
            "terminal_title_stripped": title,
            "cwd": cwd,
        })
    }

    #[test]
    fn uses_the_most_recently_active_agent_title() {
        let agents = [
            agent("w1:t1", 3, "Old task", "/code/app"),
            agent("w1:t1", 9, "PR 457", "/code/app"),
            agent("w1:t2", 1, "Other", "/code/app"),
        ];
        let wanted = wanted_labels(&agents);
        assert_eq!(wanted["w1:t1"], "PR 457");
        assert_eq!(wanted["w1:t2"], "Other");
    }

    #[test]
    fn falls_back_to_directory_for_generic_titles() {
        for title in ["", "Claude Code", "claude"] {
            let agents = [agent("w1:t1", 1, title, "/code/meety/")];
            let wanted = wanted_labels(&agents);
            assert_eq!(wanted["w1:t1"], "meety");
        }
    }

    #[test]
    fn truncates_long_titles() {
        let label = truncate("Turborepo Next Kysely monorepo (2)");
        assert_eq!(label, "Turborepo Next Kysely m…");
        assert_eq!(label.chars().count(), MAX_LABEL_CHARS);
    }

    #[test]
    fn only_numeric_labels_are_default() {
        assert!(is_default_label("3"));
        assert!(!is_default_label("logs"));
        assert!(!is_default_label(""));
    }
}
