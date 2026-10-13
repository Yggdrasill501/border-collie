//! Keeps spaces ordered by most recently focused, current space on top.

use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::herdr::{self, Result};

/// Returns the focused space's id and its position in the list.
fn focused() -> Result<Option<(String, usize)>> {
    let spaces = herdr::list("workspace.list", "workspaces")?;
    Ok(spaces.iter().enumerate().find_map(|(index, space)| {
        let id = space["workspace_id"].as_str()?;
        (space["focused"] == true).then(|| (id.to_string(), index))
    }))
}

/// Moves the focused space to the top if it is still focused after `dwell`,
/// so spaces passed through while navigating are not reshuffled.
pub fn promote_focused(dwell: Duration) -> Result<()> {
    let Some((before, _)) = focused()? else {
        return Ok(());
    };
    thread::sleep(dwell);
    match focused()? {
        Some((id, index)) if id == before && index > 0 => {
            herdr::request(
                "workspace.move",
                json!({ "workspace_id": id, "insert_index": 0 }),
            )?;
            println!("moved {id} to top");
        }
        _ => {}
    }
    Ok(())
}
