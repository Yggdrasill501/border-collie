//! Names tabs after the most recently active agent they contain.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::herdr::{self, Result};
use crate::label;
use crate::state::Names;

/// Herdr labels unnamed tabs with their number.
fn is_default_label(label: &str) -> bool {
    !label.is_empty() && label.chars().all(|c| c.is_ascii_digit())
}

/// Picks one label per tab from its most recently active agent.
fn wanted_labels(agents: &[Value], max: usize) -> HashMap<&str, String> {
    let seq = |agent: &Value| agent["state_change_seq"].as_u64().unwrap_or(0);
    let mut latest: HashMap<&str, &Value> = HashMap::new();
    for agent in agents {
        let Some(tab) = agent["tab_id"].as_str() else {
            continue;
        };
        let current = latest.entry(tab).or_insert(agent);
        if seq(agent) > seq(current) {
            *current = agent;
        }
    }
    latest
        .into_iter()
        .filter_map(|(tab, agent)| Some((tab, label::truncate(label::subject(agent)?, max))))
        .collect()
}

/// Renames every tab collie may touch. `named` holds the labels collie set on
/// earlier runs and is updated in place.
pub fn rename_all(agents: &[Value], max: usize, named: &mut Names) -> Result<()> {
    let tabs = herdr::list("tab.list", "tabs")?;
    let wanted = wanted_labels(agents, max);

    named.retain(|id, _| tabs.iter().any(|tab| tab["tab_id"] == id.as_str()));
    for tab in &tabs {
        let (Some(id), Some(label)) = (tab["tab_id"].as_str(), tab["label"].as_str()) else {
            continue;
        };
        let ours = named.get(id).is_some_and(|set| set == label);
        if !(ours || is_default_label(label)) {
            // Renamed by hand: stop tracking it.
            named.remove(id);
            continue;
        }
        let Some(new) = wanted.get(id) else {
            continue;
        };
        if new == label {
            named.insert(id.to_string(), new.clone());
            continue;
        }
        match herdr::request("tab.rename", json!({ "tab_id": id, "label": new })) {
            Ok(result) => {
                // Track the label herdr stored, in case it normalised ours.
                let stored = result["tab"]["label"].as_str().unwrap_or(new);
                println!("tab {id}: {label} -> {stored}");
                named.insert(id.to_string(), stored.to_string());
            }
            Err(error) => eprintln!("collie: tab {id}: {error}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(tab: &str, seq: u64, title: &str) -> Value {
        json!({
            "agent": "claude",
            "tab_id": tab,
            "state_change_seq": seq,
            "terminal_title_stripped": title,
            "cwd": "/code/app",
        })
    }

    #[test]
    fn uses_the_most_recently_active_agent_title() {
        let agents = [
            agent("w1:t1", 3, "Old task"),
            agent("w1:t1", 9, "PR 457"),
            agent("w1:t1", 5, "Middle task"),
            agent("w1:t2", 1, "Other"),
        ];
        let wanted = wanted_labels(&agents, 24);
        assert_eq!(wanted.len(), 2);
        assert_eq!(wanted["w1:t1"], "PR 457");
        assert_eq!(wanted["w1:t2"], "Other");
    }

    #[test]
    fn labels_respect_the_configured_length() {
        let agents = [agent("w1:t1", 1, "Linear tasks needing attention")];
        assert_eq!(wanted_labels(&agents, 10)["w1:t1"], "Linear ta…");
    }

    #[test]
    fn only_numeric_labels_are_default() {
        assert!(is_default_label("3"));
        assert!(is_default_label("12"));
        assert!(!is_default_label("logs"));
        assert!(!is_default_label("v2"));
        assert!(!is_default_label(""));
    }
}
