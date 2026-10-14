//! Undoes what border-collie has set: its tab labels, agent names and pins.

use serde_json::{json, Value};

use crate::herdr::{self, Result};
use crate::state::{State, Store};

/// Tabs still carrying a label border-collie set, with the number to return
/// them to.
fn tabs_to_restore<'a>(tabs: &'a [Value], state: &State) -> Vec<(&'a str, u64)> {
    tabs.iter()
        .filter_map(|tab| {
            let id = tab["tab_id"].as_str()?;
            let ours = state.tabs.get(id)? == tab["label"].as_str()?;
            ours.then_some((id, tab["number"].as_u64()?))
        })
        .collect()
}

/// Panes of agents still carrying a name border-collie set.
fn agents_to_clear<'a>(agents: &'a [Value], state: &State) -> Vec<&'a str> {
    agents
        .iter()
        .filter_map(|agent| {
            let ours =
                state.agents.get(agent["terminal_id"].as_str()?)? == agent["name"].as_str()?;
            ours.then_some(agent["pane_id"].as_str()?)
        })
        .collect()
}

/// Returns tabs to their numbers, clears agent names and forgets pins. Names
/// set by hand are not touched. Naming resumes on the next agent event unless
/// it is switched off in the config or the plugin is uninstalled.
pub fn run() -> Result<()> {
    let store = Store::open()?;
    let state = store.load();
    let tabs = herdr::list("tab.list", "tabs")?;
    let agents = herdr::list("agent.list", "agents")?;

    for (id, number) in tabs_to_restore(&tabs, &state) {
        match herdr::request(
            "tab.rename",
            json!({ "tab_id": id, "label": number.to_string() }),
        ) {
            Ok(_) => println!("tab {id}: restored to {number}"),
            Err(error) => eprintln!("border-collie: tab {id}: {error}"),
        }
    }
    for pane in agents_to_clear(&agents, &state) {
        match herdr::request("agent.rename", json!({ "target": pane, "name": null })) {
            Ok(_) => println!("agent {pane}: name cleared"),
            Err(error) => eprintln!("border-collie: agent {pane}: {error}"),
        }
    }
    if !state.pins.is_empty() {
        println!("forgot {} pinned space(s)", state.pins.len());
    }
    store.save(&State::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        let mut state = State::default();
        state.tabs.insert("w1:t1".into(), "PR 457".into());
        state.tabs.insert("w1:t2".into(), "Old name".into());
        state.agents.insert("term_1".into(), "pr-457".into());
        state.agents.insert("term_2".into(), "old-name".into());
        state
    }

    #[test]
    fn only_tabs_still_carrying_our_label_are_restored() {
        let tabs = [
            json!({ "tab_id": "w1:t1", "label": "PR 457", "number": 1 }),
            json!({ "tab_id": "w1:t2", "label": "renamed by hand", "number": 2 }),
            json!({ "tab_id": "w1:t3", "label": "never ours", "number": 3 }),
        ];
        assert_eq!(tabs_to_restore(&tabs, &state()), [("w1:t1", 1)]);
    }

    #[test]
    fn only_agents_still_carrying_our_name_are_cleared() {
        let agents = [
            json!({ "terminal_id": "term_1", "pane_id": "w1:p1", "name": "pr-457" }),
            json!({ "terminal_id": "term_2", "pane_id": "w1:p2", "name": "reviewer" }),
            json!({ "terminal_id": "term_3", "pane_id": "w1:p3", "name": "manual" }),
            json!({ "terminal_id": "term_4", "pane_id": "w1:p4" }),
        ];
        assert_eq!(agents_to_clear(&agents, &state()), ["w1:p1"]);
    }
}
