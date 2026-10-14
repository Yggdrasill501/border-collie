//! Orders spaces: pinned spaces hold the top slots, the rest follow with the
//! most recently focused first.

use std::env;
use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::herdr::{self, Result};
use crate::state::Store;

struct Space {
    id: String,
    label: String,
    focused: bool,
}

fn spaces() -> Result<Vec<Space>> {
    let spaces = herdr::list("workspace.list", "workspaces")?;
    Ok(spaces
        .iter()
        .filter_map(|space| {
            Some(Space {
                id: space["workspace_id"].as_str()?.to_string(),
                label: space["label"].as_str().unwrap_or("").to_string(),
                focused: space["focused"] == true,
            })
        })
        .collect())
}

fn focused_id(spaces: &[Space]) -> Option<&str> {
    spaces
        .iter()
        .find(|space| space.focused)
        .map(|space| space.id.as_str())
}

/// The order spaces should be in: pins first, in slot order, then `recent`
/// (the space to promote, unless it is pinned), then everything else as it
/// was.
fn desired_order<'a>(current: &[&'a str], pins: &[String], recent: Option<&str>) -> Vec<&'a str> {
    let find = |id: &str| current.iter().copied().find(|space| *space == id);
    let is_pinned = |id: &str| pins.iter().any(|pin| pin == id);

    let mut order: Vec<&str> = pins.iter().filter_map(|pin| find(pin)).collect();
    order.extend(recent.filter(|id| !is_pinned(id)).and_then(find));
    for space in current {
        if !order.contains(space) {
            order.push(space);
        }
    }
    order
}

/// The `workspace.move` calls that turn `current` into `desired`, applied in
/// order. Each entry is a space and the index to move it to.
fn moves<'a>(current: &[&'a str], desired: &[&'a str]) -> Vec<(&'a str, usize)> {
    let mut current = current.to_vec();
    let mut moves = Vec::new();
    for (index, space) in desired.iter().enumerate() {
        if current.get(index) == Some(space) {
            continue;
        }
        current.retain(|other| other != space);
        current.insert(index.min(current.len()), space);
        moves.push((*space, index));
    }
    moves
}

/// Reorders spaces to match the pins, promoting `recent` if given.
fn arrange(spaces: &[Space], pins: &[String], recent: Option<&str>) -> Result<()> {
    let current: Vec<&str> = spaces.iter().map(|space| space.id.as_str()).collect();
    let desired = desired_order(&current, pins, recent);
    for (id, index) in moves(&current, &desired) {
        herdr::request(
            "workspace.move",
            json!({ "workspace_id": id, "insert_index": index }),
        )?;
        println!("space {id}: moved to position {}", index + 1);
    }
    Ok(())
}

/// Forgets pins whose space no longer exists.
fn live_pins(pins: &[String], spaces: &[Space]) -> Vec<String> {
    pins.iter()
        .filter(|pin| spaces.iter().any(|space| &space.id == *pin))
        .cloned()
        .collect()
}

/// Moves the focused space up if it is still focused after `dwell`, so
/// spaces passed through while navigating are not reshuffled.
pub fn promote_focused(dwell: Duration) -> Result<()> {
    let before = spaces()?;
    let Some(before) = focused_id(&before) else {
        return Ok(());
    };
    thread::sleep(dwell);

    let store = Store::open()?;
    let spaces = spaces()?;
    let pins = live_pins(&store.load().pins, &spaces);
    let recent = focused_id(&spaces).filter(|id| *id == before);
    arrange(&spaces, &pins, recent)
}

fn notify(title: &str) {
    let shown = herdr::request("notification.show", json!({ "title": title }));
    if shown.is_err() {
        println!("{title}");
    }
}

/// Pins the current space to the next free slot, or unpins it.
pub fn toggle_pin() -> Result<()> {
    let store = Store::open()?;
    let mut state = store.load();
    let spaces = spaces()?;
    // Prefer the space the action was invoked from over the focused one.
    let invoked = env::var("HERDR_WORKSPACE_ID").ok();
    let target = invoked
        .as_deref()
        .and_then(|id| spaces.iter().find(|space| space.id == id))
        .or_else(|| spaces.iter().find(|space| space.focused))
        .ok_or("no space to pin")?;

    let mut pins = live_pins(&state.pins, &spaces);
    let message = match pins.iter().position(|pin| pin == &target.id) {
        Some(slot) => {
            pins.remove(slot);
            format!("Unpinned {}", target.label)
        }
        None => {
            pins.push(target.id.clone());
            format!("Pinned {} to slot {}", target.label, pins.len())
        }
    };
    state.pins = pins;
    store.save(&state)?;
    arrange(&spaces, &state.pins, None)?;
    notify(&message);
    Ok(())
}

/// Describes the pinned spaces, for `border-collie pins`.
pub fn list_pins() -> Result<()> {
    let store = Store::open()?;
    let spaces = spaces()?;
    let pins = live_pins(&store.load().pins, &spaces);
    if pins.is_empty() {
        println!("no pinned spaces");
    }
    for (slot, pin) in pins.iter().enumerate() {
        let label = spaces
            .iter()
            .find(|space| &space.id == pin)
            .map_or("", |space| space.label.as_str());
        println!("{}  {label}  ({pin})", slot + 1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pins(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// Applies moves the way herdr does: remove, then insert at the index.
    fn apply<'a>(current: &[&'a str], moves: &[(&'a str, usize)]) -> Vec<&'a str> {
        let mut order = current.to_vec();
        for (space, index) in moves {
            order.retain(|other| other != space);
            order.insert(*index, space);
        }
        order
    }

    #[test]
    fn the_recent_space_moves_to_the_top() {
        let current = ["a", "b", "c", "d"];
        assert_eq!(
            desired_order(&current, &[], Some("c")),
            ["c", "a", "b", "d"]
        );
        assert_eq!(desired_order(&current, &[], None), current);
    }

    #[test]
    fn pins_hold_the_top_slots_in_pin_order() {
        let current = ["a", "b", "c", "d"];
        let pinned = pins(&["d", "b"]);
        assert_eq!(
            desired_order(&current, &pinned, Some("c")),
            ["d", "b", "c", "a"]
        );
    }

    #[test]
    fn focusing_a_pinned_space_moves_nothing() {
        let current = ["d", "b", "c", "a"];
        let pinned = pins(&["d", "b"]);
        assert_eq!(desired_order(&current, &pinned, Some("b")), current);
    }

    #[test]
    fn unknown_ids_are_ignored() {
        let current = ["a", "b"];
        let pinned = pins(&["gone"]);
        assert_eq!(desired_order(&current, &pinned, Some("missing")), current);
    }

    #[test]
    fn moves_reach_the_desired_order() {
        let cases: [(&[&str], &[&str]); 4] = [
            (&["a", "b", "c", "d"], &["c", "a", "b", "d"]),
            (&["a", "b", "c", "d"], &["d", "b", "c", "a"]),
            (&["a", "b", "c", "d"], &["d", "c", "b", "a"]),
            (&["a"], &["a"]),
        ];
        for (current, desired) in cases {
            assert_eq!(apply(current, &moves(current, desired)), desired);
        }
    }

    #[test]
    fn promoting_one_space_takes_one_move() {
        let current = ["a", "b", "c", "d"];
        assert_eq!(moves(&current, &["c", "a", "b", "d"]), [("c", 0)]);
        assert!(moves(&current, &current).is_empty());
    }
}
