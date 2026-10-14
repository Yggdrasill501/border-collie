//! Interactive agent picker, shown in a herdr popup.
//!
//! Lists agents most recently active first. Press a digit to jump straight
//! to an agent, or move with j/k or the arrow keys and confirm with enter.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

use crate::herdr::{self, Result};
use crate::label;

const PLUGIN_ID: &str = "border-collie";
const ENTRYPOINT: &str = "agents";

/// Asks herdr to open the picker popup. Bound to a key through the
/// `pick-agent` plugin action.
pub fn open() -> Result<()> {
    herdr::request(
        "plugin.pane.open",
        json!({ "plugin_id": PLUGIN_ID, "entrypoint": ENTRYPOINT }),
    )?;
    Ok(())
}

struct Row {
    pane: String,
    status: String,
    space: String,
    label: String,
}

/// Agents, most recently active first.
fn rows(agents: &[Value], spaces: &[Value]) -> Vec<Row> {
    let labels: HashMap<&str, &str> = spaces
        .iter()
        .filter_map(|space| Some((space["workspace_id"].as_str()?, space["label"].as_str()?)))
        .collect();
    let mut agents: Vec<&Value> = agents.iter().collect();
    agents.sort_by_key(|agent| std::cmp::Reverse(agent["state_change_seq"].as_u64().unwrap_or(0)));
    agents
        .into_iter()
        .filter_map(|agent| {
            let space = agent["workspace_id"].as_str().unwrap_or("");
            Some(Row {
                pane: agent["pane_id"].as_str()?.to_string(),
                status: agent["agent_status"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                space: labels.get(space).copied().unwrap_or(space).to_string(),
                label: label::subject(agent)
                    .or(agent["agent"].as_str())
                    .unwrap_or("agent")
                    .to_string(),
            })
        })
        .collect()
}

#[derive(Debug, PartialEq)]
enum Key {
    Up,
    Down,
    Confirm,
    Cancel,
    Digit(usize),
    Other,
}

#[derive(Debug, PartialEq)]
enum Step {
    Select(usize),
    Choose(usize),
    Cancel,
}

/// Applies a key to the current selection in a list of `len` rows.
fn step(selected: usize, len: usize, key: Key) -> Step {
    match key {
        Key::Up => Step::Select(selected.saturating_sub(1)),
        Key::Down => Step::Select((selected + 1).min(len.saturating_sub(1))),
        Key::Confirm if len > 0 => Step::Choose(selected),
        Key::Digit(n) if (1..=len).contains(&n) => Step::Choose(n - 1),
        Key::Cancel => Step::Cancel,
        _ => Step::Select(selected),
    }
}

/// First visible row so that `selected` stays on screen.
fn scroll_offset(selected: usize, offset: usize, visible: usize) -> usize {
    if selected < offset {
        selected
    } else if visible > 0 && selected >= offset + visible {
        selected + 1 - visible
    } else {
        offset
    }
}

fn status_mark(status: &str) -> (&'static str, &'static str) {
    match status {
        "working" => ("●", "33"),
        "blocked" => ("!", "31"),
        "done" => ("✓", "32"),
        "idle" => ("·", "2"),
        _ => ("?", "2"),
    }
}

/// Width of the number and status mark columns that start each line.
const GUTTER: usize = 5;

/// The text of a list line after the gutter, cut to fit `width` columns.
fn format_row(row: &Row, width: usize) -> String {
    let text = format!(
        " {:<8} {:<18} {}",
        row.status,
        label::truncate(&row.space, 18),
        row.label
    );
    label::truncate(&text, width.saturating_sub(GUTTER))
}

fn draw(
    out: &mut impl Write,
    rows: &[Row],
    selected: usize,
    offset: usize,
    size: (usize, usize),
) -> io::Result<()> {
    let (height, width) = size;
    let title = if rows.is_empty() {
        "No agents running"
    } else {
        "Jump to agent"
    };
    write!(out, "\x1b[H\x1b[2J\x1b[2m {title}\x1b[0m\r\n")?;
    let visible = height.saturating_sub(2);
    for (index, row) in rows.iter().enumerate().skip(offset).take(visible) {
        let number = match index {
            0..=8 => (index + 1).to_string(),
            _ => " ".to_string(),
        };
        let (mark, colour) = status_mark(&row.status);
        let text = format_row(row, width);
        if index == selected {
            write!(out, "\x1b[7m {number}  {mark}{text}\x1b[0m\r\n")?;
        } else {
            write!(out, " {number}  \x1b[{colour}m{mark}\x1b[0m{text}\r\n")?;
        }
    }
    write!(
        out,
        "\x1b[2m 1-9 jump · j/k move · enter select · esc cancel\x1b[0m"
    )?;
    out.flush()
}

fn stty(args: &[&str]) -> Option<String> {
    let output = Command::new("stty")
        .args(args)
        .stdin(Stdio::inherit())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Puts the terminal in single-key mode and restores it when dropped.
struct RawMode {
    saved: String,
}

impl RawMode {
    fn enter() -> Result<Self> {
        let saved = stty(&["-g"]).ok_or("the agent picker needs a terminal")?;
        // Reads time out after 0.1s so a lone escape can be told from an arrow key.
        stty(&["-icanon", "-echo", "-isig", "min", "0", "time", "1"])
            .ok_or("could not configure the terminal")?;
        print!("\x1b[?25l");
        Ok(Self { saved })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        stty(&[&self.saved]);
        print!("\x1b[?25h\x1b[H\x1b[2J");
        let _ = io::stdout().flush();
    }
}

/// Terminal size as (rows, columns).
fn terminal_size() -> (usize, usize) {
    stty(&["size"])
        .and_then(|size| {
            let (rows, columns) = size.split_once(' ')?;
            Some((rows.parse().ok()?, columns.parse().ok()?))
        })
        .unwrap_or((24, 80))
}

/// Reads one byte, or `None` when the read timed out.
fn read_byte(input: &mut impl Read) -> io::Result<Option<u8>> {
    let mut byte = [0u8];
    Ok((input.read(&mut byte)? == 1).then_some(byte[0]))
}

/// Reads the rest of an escape sequence and returns its final byte, so that
/// parameters such as the `1;5` in ctrl+arrow are not taken for key presses.
fn read_sequence_end(input: &mut impl Read) -> io::Result<Option<u8>> {
    loop {
        match read_byte(input)? {
            Some(byte) if (0x40..=0x7e).contains(&byte) => return Ok(Some(byte)),
            Some(_) => continue,
            None => return Ok(None),
        }
    }
}

/// Waits for the next key press.
fn read_key(input: &mut impl Read) -> io::Result<Key> {
    loop {
        let Some(byte) = read_byte(input)? else {
            continue;
        };
        return Ok(match byte {
            b'k' => Key::Up,
            b'j' => Key::Down,
            b'\r' | b'\n' => Key::Confirm,
            b'q' | 0x03 => Key::Cancel,
            b'1'..=b'9' => Key::Digit(usize::from(byte - b'0')),
            0x1b => match read_byte(input)? {
                None => Key::Cancel,
                Some(b'[') | Some(b'O') => match read_sequence_end(input)? {
                    Some(b'A') => Key::Up,
                    Some(b'B') => Key::Down,
                    _ => Key::Other,
                },
                Some(_) => Key::Other,
            },
            _ => Key::Other,
        });
    }
}

/// Runs the picker in the current terminal and focuses the chosen agent.
pub fn run() -> Result<()> {
    let agents = herdr::list("agent.list", "agents")?;
    let spaces = herdr::list("workspace.list", "workspaces")?;
    let rows = rows(&agents, &spaces);

    let raw = RawMode::enter()?;
    let size = terminal_size();
    let mut input = io::stdin().lock();
    let mut out = io::stdout().lock();
    let (mut selected, mut offset) = (0, 0);
    let chosen = loop {
        offset = scroll_offset(selected, offset, size.0.saturating_sub(2));
        draw(&mut out, &rows, selected, offset, size)?;
        match step(selected, rows.len(), read_key(&mut input)?) {
            Step::Select(index) => selected = index,
            Step::Choose(index) => break Some(index),
            Step::Cancel => break None,
        }
    };
    drop(out);
    drop(raw);

    if let Some(row) = chosen.and_then(|index| rows.get(index)) {
        herdr::request("agent.focus", json!({ "target": row.pane }))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(pane: &str, space: &str, seq: u64, title: &str) -> Value {
        json!({
            "agent": "claude",
            "pane_id": pane,
            "workspace_id": space,
            "agent_status": "idle",
            "state_change_seq": seq,
            "terminal_title_stripped": title,
            "cwd": "/code/app",
        })
    }

    #[test]
    fn rows_are_most_recent_first_with_space_labels() {
        let agents = [
            agent("w1:p1", "w1", 3, "Old"),
            agent("w2:p1", "w2", 9, "New"),
            agent("w3:p1", "w3", 5, "Claude Code"),
        ];
        let spaces = [
            json!({ "workspace_id": "w1", "label": "meety" }),
            json!({ "workspace_id": "w2", "label": "docs" }),
        ];
        let rows = rows(&agents, &spaces);
        let summary: Vec<_> = rows
            .iter()
            .map(|row| (row.pane.as_str(), row.space.as_str(), row.label.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                ("w2:p1", "docs", "New"),
                ("w3:p1", "w3", "app"),
                ("w1:p1", "meety", "Old")
            ]
        );
    }

    #[test]
    fn digits_choose_only_existing_rows() {
        assert_eq!(step(0, 3, Key::Digit(3)), Step::Choose(2));
        assert_eq!(step(1, 3, Key::Digit(4)), Step::Select(1));
    }

    #[test]
    fn movement_stops_at_the_ends() {
        assert_eq!(step(0, 3, Key::Up), Step::Select(0));
        assert_eq!(step(1, 3, Key::Down), Step::Select(2));
        assert_eq!(step(2, 3, Key::Down), Step::Select(2));
    }

    #[test]
    fn an_empty_list_can_only_be_cancelled() {
        assert_eq!(step(0, 0, Key::Confirm), Step::Select(0));
        assert_eq!(step(0, 0, Key::Down), Step::Select(0));
        assert_eq!(step(0, 0, Key::Digit(1)), Step::Select(0));
        assert_eq!(step(0, 0, Key::Cancel), Step::Cancel);
    }

    #[test]
    fn scrolling_keeps_the_selection_visible() {
        assert_eq!(scroll_offset(0, 0, 5), 0);
        assert_eq!(scroll_offset(5, 0, 5), 1);
        assert_eq!(scroll_offset(9, 1, 5), 5);
        assert_eq!(scroll_offset(2, 5, 5), 2);
        assert_eq!(scroll_offset(3, 0, 0), 0);
    }

    #[test]
    fn keys_are_decoded_from_bytes() {
        let key = |bytes: &[u8]| read_key(&mut &bytes[..]).ok();
        assert_eq!(key(b"j"), Some(Key::Down));
        assert_eq!(key(b"\x1b[A"), Some(Key::Up));
        assert_eq!(key(b"\x1bOB"), Some(Key::Down));
        assert_eq!(key(b"\r"), Some(Key::Confirm));
        assert_eq!(key(b"7"), Some(Key::Digit(7)));
        assert_eq!(key(b"\x03"), Some(Key::Cancel));
        assert_eq!(key(b"x"), Some(Key::Other));
    }

    #[test]
    fn escape_sequence_parameters_are_not_key_presses() {
        // ctrl+up, page down and a mouse report all contain digits.
        for sequence in [&b"\x1b[1;5A"[..], b"\x1b[6~", b"\x1b[<0;10;5M"] {
            let mut input = sequence;
            let first = read_key(&mut input).ok();
            assert!(matches!(first, Some(Key::Up | Key::Other)), "{sequence:?}");
            assert!(input.is_empty(), "{sequence:?} left input unread");
        }
    }

    #[test]
    fn rows_fit_the_terminal_width() {
        let row = Row {
            pane: "w1:p1".into(),
            status: "working".into(),
            space: "a-very-long-space-name-indeed".into(),
            label: "A long title that will not fit".into(),
        };
        let line = format_row(&row, 40);
        assert_eq!(line.chars().count(), 40 - GUTTER);
        assert!(line.starts_with(" working  a-very-long-space…"));
    }
}
