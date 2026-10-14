//! Names agents after what they are working on.
//!
//! Herdr agent names are unique handles matching `[a-z][a-z0-9_-]{0,31}`, so
//! an agent titled "PR 457" becomes `pr-457` and can be addressed that way:
//! `herdr agent focus pr-457`.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::herdr::{self, Result};
use crate::label::{self, MAX_AGENT_NAME_CHARS};
use crate::state::Names;

struct Agent<'a> {
    /// Stable for the life of the pane's terminal; the state key.
    terminal: &'a str,
    pane: &'a str,
    name: Option<&'a str>,
    /// Name border-collie would like the agent to have, before deduplication.
    base: String,
}

/// The base name for an agent: a slug of its title, else of its directory,
/// else its kind (`claude`, `codex`, ...).
fn base_name(agent: &Value) -> Option<String> {
    let candidates = [
        label::subject(agent),
        agent["cwd"].as_str().and_then(|cwd| cwd.rsplit('/').next()),
        agent["agent"].as_str(),
    ];
    candidates
        .into_iter()
        .flatten()
        .map(label::slug)
        .find(|slug| !slug.is_empty())
}

/// `base`, or `base-N` for N >= 2, cut so the whole name fits herdr's limit.
fn numbered(base: &str, n: usize) -> String {
    if n < 2 {
        return base.to_string();
    }
    let suffix = format!("-{n}");
    let room = MAX_AGENT_NAME_CHARS - suffix.len();
    let head = base[..base.len().min(room)].trim_end_matches('-');
    format!("{head}{suffix}")
}

/// True when `name` is `base` or one of its numbered variants.
fn is_variant(name: &str, base: &str) -> bool {
    (1..)
        .map(|n| numbered(base, n))
        .take(100)
        .any(|v| v == name)
}

/// Decides the name each agent should carry. Agents named by hand keep their
/// name and reserve it; agents already holding a fitting name keep it, so
/// names do not shuffle between runs; the rest take the first free variant.
fn plan<'a>(agents: &[Agent<'a>], named: &Names, reserved: &[&str]) -> HashMap<&'a str, String> {
    let ours = |agent: &Agent| match agent.name {
        None => true,
        Some(name) => named.get(agent.terminal).is_some_and(|set| set == name),
    };
    let mut taken: HashSet<String> = reserved.iter().map(|name| name.to_string()).collect();
    let mut pending = Vec::new();
    for agent in agents {
        match agent.name {
            Some(name) if !ours(agent) || is_variant(name, &agent.base) => {
                taken.insert(name.to_string());
            }
            _ => pending.push(agent),
        }
    }

    let mut planned = HashMap::new();
    for agent in pending {
        let free = (1..)
            .map(|n| numbered(&agent.base, n))
            .find(|name| !taken.contains(name));
        if let Some(name) = free {
            taken.insert(name.clone());
            planned.insert(agent.terminal, name);
        }
    }
    planned
}

/// Renames every agent border-collie may touch. `named` holds the names border-collie set
/// on earlier runs and is updated in place.
pub fn rename_all(agents: &[Value], named: &mut Names) -> Result<()> {
    let mut live: Vec<Agent> = agents
        .iter()
        .filter_map(|agent| {
            Some(Agent {
                terminal: agent["terminal_id"].as_str()?,
                pane: agent["pane_id"].as_str()?,
                name: agent["name"].as_str(),
                base: base_name(agent)?,
            })
        })
        .collect();
    // A fixed order keeps numbering stable from one run to the next.
    live.sort_by_key(|agent| agent.pane);

    // Names of agents border-collie cannot describe still have to stay unique.
    let reserved: Vec<&str> = agents
        .iter()
        .filter_map(|agent| agent["name"].as_str())
        .collect();
    let planned = plan(&live, named, &reserved);
    named.retain(|terminal, _| live.iter().any(|agent| agent.terminal == terminal));
    for agent in &live {
        let Some(new) = planned.get(agent.terminal) else {
            // Keeps its current name: track it if it is ours, forget it if not.
            match agent.name {
                Some(name) if named.get(agent.terminal).is_some_and(|set| set == name) => {}
                _ => {
                    named.remove(agent.terminal);
                }
            }
            continue;
        };
        match herdr::request("agent.rename", json!({ "target": agent.pane, "name": new })) {
            Ok(_) => {
                println!(
                    "agent {}: {} -> {new}",
                    agent.pane,
                    agent.name.unwrap_or("(unnamed)")
                );
                named.insert(agent.terminal.to_string(), new.clone());
            }
            Err(error) => eprintln!("border-collie: agent {}: {error}", agent.pane),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent<'a>(terminal: &'a str, name: Option<&'a str>, base: &str) -> Agent<'a> {
        Agent {
            terminal,
            pane: terminal,
            name,
            base: base.to_string(),
        }
    }

    fn names(pairs: &[(&str, &str)]) -> Names {
        pairs
            .iter()
            .map(|(key, name)| (key.to_string(), name.to_string()))
            .collect()
    }

    #[test]
    fn base_name_prefers_title_then_directory_then_kind() {
        let titled =
            json!({ "agent": "claude", "terminal_title_stripped": "PR 457", "cwd": "/code/app" });
        assert_eq!(base_name(&titled).as_deref(), Some("pr-457"));
        let generic = json!({ "agent": "claude", "terminal_title_stripped": "Claude Code", "cwd": "/code/app" });
        assert_eq!(base_name(&generic).as_deref(), Some("app"));
        let numeric =
            json!({ "agent": "claude", "terminal_title_stripped": "2024", "cwd": "/code/app" });
        assert_eq!(base_name(&numeric).as_deref(), Some("app"));
        let bare = json!({ "agent": "claude", "terminal_title_stripped": "", "cwd": "/" });
        assert_eq!(base_name(&bare).as_deref(), Some("claude"));
    }

    #[test]
    fn numbered_names_fit_the_length_limit() {
        assert_eq!(numbered("meety", 1), "meety");
        assert_eq!(numbered("meety", 2), "meety-2");
        let long = "a".repeat(MAX_AGENT_NAME_CHARS);
        let name = numbered(&long, 12);
        assert_eq!(name.len(), MAX_AGENT_NAME_CHARS);
        assert!(name.ends_with("-12"));
    }

    #[test]
    fn unnamed_agents_get_unique_names() {
        let agents = [
            agent("t1", None, "meety"),
            agent("t2", None, "meety"),
            agent("t3", None, "pr-457"),
        ];
        let planned = plan(&agents, &Names::new(), &[]);
        assert_eq!(planned["t1"], "meety");
        assert_eq!(planned["t2"], "meety-2");
        assert_eq!(planned["t3"], "pr-457");
    }

    #[test]
    fn manual_names_are_kept_and_reserved() {
        let agents = [
            agent("t1", Some("meety"), "something-else"),
            agent("t2", None, "meety"),
        ];
        let planned = plan(&agents, &Names::new(), &[]);
        assert!(!planned.contains_key("t1"));
        assert_eq!(planned["t2"], "meety-2");
    }

    #[test]
    fn reserved_names_are_never_reused() {
        let agents = [agent("t1", None, "meety")];
        let planned = plan(&agents, &Names::new(), &["meety", "meety-2"]);
        assert_eq!(planned["t1"], "meety-3");
    }

    #[test]
    fn fitting_names_do_not_shuffle() {
        let named = names(&[("t1", "meety-2"), ("t2", "meety")]);
        let agents = [
            agent("t1", Some("meety-2"), "meety"),
            agent("t2", Some("meety"), "meety"),
            agent("t3", None, "meety"),
        ];
        let planned = plan(&agents, &named, &[]);
        assert_eq!(planned.len(), 1);
        assert_eq!(planned["t3"], "meety-3");
    }

    #[test]
    fn our_names_follow_the_agents_work() {
        let named = names(&[("t1", "meety")]);
        let agents = [agent("t1", Some("meety"), "pr-457")];
        assert_eq!(plan(&agents, &named, &[])["t1"], "pr-457");
    }

    #[test]
    fn a_name_set_by_hand_after_ours_is_left_alone() {
        let named = names(&[("t1", "meety")]);
        let agents = [agent("t1", Some("reviewer"), "pr-457")];
        assert!(plan(&agents, &named, &[]).is_empty());
    }
}
