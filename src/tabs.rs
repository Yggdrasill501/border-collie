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

#[derive(Debug, PartialEq)]
enum Decision {
    /// Not ours to rename, or nothing to rename it to.
    Leave,
    /// Already carries the label we want.
    Keep,
    Rename(String),
}

/// Decides what to do with one tab. `ours` says whether its current label
/// was set by collie; `wanted` is the label its agents suggest, if it has
/// any.
fn decide(label: &str, number: u64, ours: bool, wanted: Option<&String>) -> Decision {
    if !(ours || is_default_label(label)) {
        return Decision::Leave;
    }
    match wanted {
        Some(new) if new == label => Decision::Keep,
        Some(new) => Decision::Rename(new.clone()),
        // The agents are gone: hand a label we set back to herdr's default
        // rather than leave it describing work that is no longer there.
        None if ours => Decision::Rename(number.to_string()),
        None => Decision::Leave,
    }
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
        let number = tab["number"].as_u64().unwrap_or(1);
        let new = match decide(label, number, ours, wanted.get(id)) {
            Decision::Leave => {
                named.remove(id);
                continue;
            }
            Decision::Keep => {
                named.insert(id.to_string(), label.to_string());
                continue;
            }
            Decision::Rename(new) => new,
        };
        match herdr::request("tab.rename", json!({ "tab_id": id, "label": new })) {
            Ok(result) => {
                // Track the label herdr stored, in case it normalised ours.
                let stored = result["tab"]["label"].as_str().unwrap_or(&new);
                println!("tab {id}: {label} -> {stored}");
                if is_default_label(stored) {
                    named.remove(id);
                } else {
                    named.insert(id.to_string(), stored.to_string());
                }
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
    fn default_and_our_labels_follow_the_agents() {
        let wanted = "PR 457".to_string();
        let rename = Decision::Rename(wanted.clone());
        assert_eq!(decide("2", 2, false, Some(&wanted)), rename);
        assert_eq!(decide("Old task", 2, true, Some(&wanted)), rename);
        assert_eq!(decide("PR 457", 2, true, Some(&wanted)), Decision::Keep);
    }

    #[test]
    fn labels_set_by_hand_are_left_alone() {
        let wanted = "PR 457".to_string();
        assert_eq!(decide("logs", 2, false, Some(&wanted)), Decision::Leave);
        assert_eq!(decide("logs", 2, false, None), Decision::Leave);
    }

    #[test]
    fn our_label_is_handed_back_when_the_agents_leave() {
        assert_eq!(
            decide("PR 457", 3, true, None),
            Decision::Rename("3".into())
        );
        assert_eq!(decide("3", 3, false, None), Decision::Leave);
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
