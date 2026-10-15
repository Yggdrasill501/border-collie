//! Jumps to the agent that most needs a look.

use serde_json::{json, Value};

use crate::herdr::{self, Result};
use crate::label;

/// How urgently an agent wants attention; lower sorts first.
fn urgency(agent: &Value) -> Option<u8> {
    match agent["agent_status"].as_str()? {
        // Waiting on an approval or an answer.
        "blocked" => Some(0),
        // Finished, and not looked at yet.
        "done" => Some(1),
        _ => None,
    }
}

/// The agent to jump to: blocked agents before finished ones, the most
/// recently changed first. The focused agent is skipped so that repeating the
/// action walks through everything that is waiting.
fn next(agents: &[Value]) -> Option<&Value> {
    agents
        .iter()
        .filter(|agent| agent["focused"] != true)
        .filter_map(|agent| Some((urgency(agent)?, agent)))
        .min_by_key(|(urgency, agent)| {
            let seq = agent["state_change_seq"].as_u64().unwrap_or(0);
            (*urgency, std::cmp::Reverse(seq))
        })
        .map(|(_, agent)| agent)
}

pub fn jump() -> Result<()> {
    let agents = herdr::agents()?;
    let Some(agent) = next(&agents) else {
        let _ = herdr::request(
            "notification.show",
            json!({ "title": "No agents need attention" }),
        );
        println!("no agents need attention");
        return Ok(());
    };
    let pane = agent["pane_id"].as_str().ok_or("agent has no pane")?;
    herdr::request("agent.focus", json!({ "target": pane }))?;
    println!(
        "focused {pane} ({})",
        label::subject(agent).unwrap_or("agent")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(pane: &str, status: &str, seq: u64, focused: bool) -> Value {
        json!({
            "pane_id": pane,
            "agent_status": status,
            "state_change_seq": seq,
            "focused": focused,
        })
    }

    fn pane(agent: Option<&Value>) -> Option<&str> {
        agent?["pane_id"].as_str()
    }

    #[test]
    fn blocked_agents_come_before_finished_ones() {
        let agents = [
            agent("done-new", "done", 9, false),
            agent("blocked-old", "blocked", 1, false),
            agent("working", "working", 20, false),
        ];
        assert_eq!(pane(next(&agents)), Some("blocked-old"));
    }

    #[test]
    fn the_most_recent_change_wins_within_a_status() {
        let agents = [
            agent("a", "done", 3, false),
            agent("b", "done", 7, false),
            agent("c", "done", 5, false),
        ];
        assert_eq!(pane(next(&agents)), Some("b"));
    }

    #[test]
    fn the_focused_agent_is_skipped() {
        let agents = [
            agent("here", "blocked", 9, true),
            agent("there", "blocked", 2, false),
        ];
        assert_eq!(pane(next(&agents)), Some("there"));
    }

    #[test]
    fn nothing_to_do_when_everyone_is_idle_or_working() {
        let agents = [
            agent("a", "idle", 3, false),
            agent("b", "working", 7, false),
            agent("c", "unknown", 5, false),
            agent("d", "blocked", 9, true),
        ];
        assert_eq!(pane(next(&agents)), None);
        assert_eq!(pane(next(&[])), None);
    }
}
