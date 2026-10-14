//! Turns what an agent is doing into short names.

use serde_json::Value;

/// Longest agent name herdr accepts.
pub const MAX_AGENT_NAME_CHARS: usize = 32;

/// What an agent is working on: its terminal title, or the working directory
/// name when the title says nothing specific.
pub fn subject(agent: &Value) -> Option<&str> {
    let title = agent["terminal_title_stripped"]
        .as_str()
        .unwrap_or("")
        .trim();
    let kind = agent["agent"].as_str().unwrap_or("");
    if !is_generic(title, kind) {
        return Some(title);
    }
    let cwd = agent["cwd"].as_str()?.trim_end_matches('/');
    cwd.rsplit('/').next().filter(|dir| !dir.is_empty())
}

/// True for titles that only name the program, such as `Claude Code` or
/// `Gemini CLI`, and so say nothing about the work.
fn is_generic(title: &str, kind: &str) -> bool {
    let rest = match title.get(..kind.len()) {
        Some(head) if head.eq_ignore_ascii_case(kind) => title[kind.len()..].trim(),
        _ => return title.is_empty(),
    };
    ["", "code", "cli"]
        .iter()
        .any(|suffix| rest.eq_ignore_ascii_case(suffix))
}

/// Shortens `label` to at most `max` characters, marking the cut with `…`.
pub fn truncate(label: &str, max: usize) -> String {
    if label.chars().count() <= max {
        return label.to_string();
    }
    let head: String = label.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", head.trim_end())
}

/// Reduces `text` to herdr's agent name alphabet, `[a-z][a-z0-9_-]{0,31}`.
/// Returns an empty string when nothing usable is left.
pub fn slug(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || (c.is_ascii_digit() && !slug.is_empty()) {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
        if slug.len() == MAX_AGENT_NAME_CHARS {
            break;
        }
    }
    slug.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn agent(title: &str, cwd: &str) -> Value {
        json!({ "agent": "claude", "terminal_title_stripped": title, "cwd": cwd })
    }

    #[test]
    fn subject_prefers_a_specific_title() {
        assert_eq!(subject(&agent("PR 457", "/code/app")), Some("PR 457"));
    }

    #[test]
    fn subject_falls_back_to_directory_for_generic_titles() {
        for title in [
            "",
            "  ",
            "Claude Code",
            "claude",
            "Claude CLI",
            "CLAUDE  code",
        ] {
            assert_eq!(subject(&agent(title, "/code/meety/")), Some("meety"));
        }
    }

    #[test]
    fn titles_that_merely_start_with_the_agent_kind_are_kept() {
        for title in ["Claude Code review of PR 4", "claudette", "Codex"] {
            assert_eq!(subject(&agent(title, "/code/meety")), Some(title));
        }
    }

    #[test]
    fn subject_is_none_without_title_or_directory() {
        assert_eq!(subject(&agent("", "/")), None);
        assert_eq!(subject(&json!({ "agent": "claude" })), None);
    }

    #[test]
    fn truncate_marks_the_cut() {
        let label = truncate("Turborepo Next Kysely monorepo (2)", 24);
        assert_eq!(label, "Turborepo Next Kysely m…");
        assert_eq!(label.chars().count(), 24);
        assert_eq!(truncate("short", 24), "short");
    }

    #[test]
    fn truncate_counts_characters_not_bytes() {
        assert_eq!(truncate("žluťoučký kůň", 6), "žluťo…");
    }

    #[test]
    fn slug_matches_the_agent_name_alphabet() {
        assert_eq!(slug("PR 457"), "pr-457");
        assert_eq!(
            slug("Linear MCP connection setup"),
            "linear-mcp-connection-setup"
        );
        assert_eq!(slug("  --Fix: the_thing!! "), "fix-the-thing");
    }

    #[test]
    fn slug_never_starts_with_a_digit_or_dash() {
        assert_eq!(slug("457 fix login"), "fix-login");
        assert_eq!(slug("2024"), "");
        assert_eq!(slug("✳ ✳"), "");
    }

    #[test]
    fn slug_stays_within_the_length_limit() {
        let name = slug("Turborepo Next Kysely monorepo with a very long title");
        assert!(name.len() <= MAX_AGENT_NAME_CHARS);
        assert!(!name.ends_with('-'));
        assert_eq!(name, "turborepo-next-kysely-monorepo-w");
    }
}
