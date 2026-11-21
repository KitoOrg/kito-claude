//! What Claude Code reads in a project, written for Kito and taken out
//! again.
//!
//! - `CLAUDE.md`: the rules, under their heading, after whatever the
//!   project says there itself.
//! - `.mcp.json`: Kito's tools (`mcpServers.kito`).
//! - `.claude/settings.json`: the permission to use those tools, and what
//!   Claude Code runs before an edit, after a shell command and when a
//!   session starts.
//!
//! What is written is only added to what is there, and taking it out
//! leaves the rest. The words are those of `kito claude`.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// One thing that was done to a file.
pub struct Done {
    pub what: &'static str,
    pub path: PathBuf,
}

/// The rules, as `CLAUDE.md` gives them to Claude: Kito's own text.
pub const RULES: &str = include_str!("../rules.md");
/// How the rules begin: a `CLAUDE.md` that has this line has them.
const HEADING: &str = "## Version control: Kito";
/// The MCP server: Claude Code calls its tools `mcp__kito__open_lane`.
const SERVER: &str = "kito";
const SERVER_PROGRAM: &str = "kito-mcp";
/// The permission for all the tools of the server, and its older form.
const ALLOW: &str = "mcp__kito__*";
const ALLOW_BARE: &str = "mcp__kito";
/// How long a hook may take, in seconds, before Claude Code goes on without it.
const HOOK_TIMEOUT: u64 = 30;
/// The hooks: the event, the tools it is for, the command.
const HOOKS: [(&str, Option<&str>, &str); 3] = [
    (
        "PreToolUse",
        Some("Edit|Write|MultiEdit|NotebookEdit"),
        "kito hook before-edit",
    ),
    ("PostToolUse", Some("Bash"), "kito hook after-shell"),
    ("SessionStart", None, "kito hook session-start"),
];

fn read(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Writes `text` to `path`, or removes the file when nothing is left to
/// say in it. `None`: it was that way already.
fn write(path: &Path, was: Option<&str>, text: &str) -> Result<Option<Done>, String> {
    let said = |e: std::io::Error| format!("{}: {e}", path.display());
    if was == Some(text) || (was.is_none() && text.is_empty()) {
        return Ok(None);
    }
    if text.trim().is_empty() {
        std::fs::remove_file(path).map_err(said)?;
        // Its directory with it, when it was the last file there.
        if let Some(dir) = path.parent().filter(|dir| dir.ends_with(".claude")) {
            let _ = std::fs::remove_dir(dir);
        }
        return Ok(Some(Done {
            what: "removed",
            path: path.to_owned(),
        }));
    }
    std::fs::create_dir_all(path.parent().unwrap_or(path)).map_err(said)?;
    std::fs::write(path, text).map_err(said)?;
    let what = if was.is_some() { "changed" } else { "wrote" };
    Ok(Some(Done {
        what,
        path: path.to_owned(),
    }))
}

/// How the part Kito writes from its instructions begins and ends
/// (Kito 0.27): all of it is Kito's, and goes with the rules.
const MARK: &str = "<!-- kito: begin";
const END: &str = "<!-- kito: end -->";

/// `text` without the part between Kito's marks.
fn without_marked(text: &str) -> String {
    let mut rest = text.to_owned();
    while let Some(from) = rest.find(MARK) {
        let to = match rest[from..].find(END) {
            Some(to) => from + to + END.len(),
            None => rest.len(),
        };
        rest.replace_range(from..to, "");
    }
    match rest.trim().is_empty() {
        true => String::new(),
        false => format!("{}\n", rest.trim()),
    }
}

/// `text` without the rules: from their heading to the next heading of
/// the same kind, or to the end; and without what Kito wrote between its
/// marks.
fn without_rules(text: &str) -> String {
    if text.contains(MARK) {
        return without_marked(text);
    }
    let Some(from) = text.lines().position(|line| line.trim_end() == HEADING) else {
        return text.to_owned();
    };
    let lines: Vec<&str> = text.lines().collect();
    let after = lines[from + 1..]
        .iter()
        .position(|line| line.starts_with("## ") || line.starts_with("# "));
    let before = lines[..from].join("\n");
    let after = after
        .map(|at| lines[from + 1 + at..].join("\n"))
        .unwrap_or_default();
    match (before.trim().is_empty(), after.trim().is_empty()) {
        (true, true) => String::new(),
        (true, false) => format!("{}\n", after.trim_end()),
        (false, true) => format!("{}\n", before.trim_end()),
        (false, false) => format!("{}\n\n{}\n", before.trim_end(), after.trim_end()),
    }
}

fn rules(path: &Path, wanted: bool) -> Result<Option<Done>, String> {
    let was = read(path)?;
    let now = was.as_deref().unwrap_or_default();
    // Rules that are there are Kito's to bring up to date (`kito claude`
    // knows the ones it wrote before): here they are there, or not.
    if wanted && now.lines().any(|line| line.trim_end() == HEADING) {
        return Ok(None);
    }
    let rest = without_rules(now);
    let text = match wanted {
        false => rest,
        true if rest.trim().is_empty() => RULES.to_owned(),
        true => format!("{}\n\n{RULES}", rest.trim_end()),
    };
    write(path, was.as_deref(), &text)
}

/// Reads a file of settings: an object, or nothing yet.
fn settings_of(path: &Path, by_hand: &str) -> Result<(Option<String>, Map<String, Value>), String> {
    let was = read(path)?;
    let all = match was.as_deref().filter(|text| !text.trim().is_empty()) {
        Some(text) => serde_json::from_str::<Value>(text)
            .map_err(|e| format!("{} is not JSON ({e}): {by_hand}", path.display()))?,
        None => json!({}),
    };
    match all {
        Value::Object(all) => Ok((was, all)),
        _ => Err(format!("{} is not an object: {by_hand}", path.display())),
    }
}

fn written(
    path: &Path,
    was: Option<String>,
    all: Map<String, Value>,
) -> Result<Option<Done>, String> {
    let text = match all.is_empty() {
        true => String::new(),
        false => {
            serde_json::to_string_pretty(&Value::Object(all)).map_err(|e| e.to_string())? + "\n"
        }
    };
    // The same settings written another way are the same settings.
    let same = was
        .as_deref()
        .and_then(|was| serde_json::from_str::<Value>(was).ok())
        == serde_json::from_str::<Value>(&text).ok();
    if same && was.is_some() && !text.is_empty() {
        return Ok(None);
    }
    write(path, was.as_deref(), &text)
}

fn server(path: &Path, wanted: bool) -> Result<Option<Done>, String> {
    let by_hand = format!(
        "add \"{SERVER}\": {{ \"command\": \"{SERVER_PROGRAM}\" }} to its mcpServers by hand"
    );
    let (was, mut all) = settings_of(path, &by_hand)?;
    let wrong = || {
        format!(
            "{}: its mcpServers is not an object: {by_hand}",
            path.display()
        )
    };
    if wanted {
        let servers = all
            .entry("mcpServers")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(wrong)?;
        // One that is there is the project's own business, whatever it runs.
        if !servers.contains_key(SERVER) {
            servers.insert(SERVER.to_owned(), json!({ "command": SERVER_PROGRAM }));
        }
    } else if let Some(servers) = all.get_mut("mcpServers").and_then(Value::as_object_mut) {
        if servers
            .get(SERVER)
            .is_some_and(|server| server["command"] == SERVER_PROGRAM)
        {
            servers.remove(SERVER);
        }
        if servers.is_empty() {
            all.remove("mcpServers");
        }
    }
    written(path, was, all)
}

/// `true` if a group of hooks runs `command`.
fn runs(group: &Value, command: &str) -> bool {
    group["hooks"]
        .as_array()
        .is_some_and(|hooks| hooks.iter().any(|hook| hook["command"] == command))
}

fn settings(path: &Path, wanted: bool) -> Result<Option<Done>, String> {
    let by_hand = format!(
        "allow `{ALLOW}` and add the hooks (`kito hook before-edit` before an edit) by hand"
    );
    let (was, mut all) = settings_of(path, &by_hand)?;
    let wrong = || {
        format!(
            "{} is not written as Claude Code writes settings: {by_hand}",
            path.display()
        )
    };
    if wanted {
        let permissions = all
            .entry("permissions")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(wrong)?;
        let allow = permissions
            .entry("allow")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(wrong)?;
        if !allow.iter().any(|rule| rule == ALLOW || rule == ALLOW_BARE) {
            allow.push(ALLOW.into());
        }
        let events = all
            .entry("hooks")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(wrong)?;
        for (event, matcher, command) in HOOKS {
            let groups = events
                .entry(event)
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .ok_or_else(wrong)?;
            if groups.iter().any(|group| runs(group, command)) {
                continue;
            }
            let mut group = Map::new();
            if let Some(matcher) = matcher {
                group.insert("matcher".to_owned(), matcher.into());
            }
            group.insert(
                "hooks".to_owned(),
                json!([{ "type": "command", "command": command, "timeout": HOOK_TIMEOUT }]),
            );
            groups.push(Value::Object(group));
        }
    } else {
        if let Some(permissions) = all.get_mut("permissions").and_then(Value::as_object_mut) {
            if let Some(allow) = permissions.get_mut("allow").and_then(Value::as_array_mut) {
                allow.retain(|rule| rule != ALLOW && rule != ALLOW_BARE);
                if allow.is_empty() {
                    permissions.remove("allow");
                }
            }
            if permissions.is_empty() {
                all.remove("permissions");
            }
        }
        if let Some(events) = all.get_mut("hooks").and_then(Value::as_object_mut) {
            for (event, _, command) in HOOKS {
                let Some(groups) = events.get_mut(event).and_then(Value::as_array_mut) else {
                    continue;
                };
                for group in groups.iter_mut() {
                    if let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                        hooks.retain(|hook| hook["command"] != command);
                    }
                }
                groups.retain(|group| {
                    group["hooks"]
                        .as_array()
                        .is_none_or(|hooks| !hooks.is_empty())
                });
                if groups.is_empty() {
                    events.remove(event);
                }
            }
            if events.is_empty() {
                all.remove("hooks");
            }
        }
    }
    written(path, was, all)
}

fn all(root: &Path, wanted: bool) -> Result<Vec<Done>, String> {
    let done = [
        rules(&root.join("CLAUDE.md"), wanted)?,
        server(&root.join(".mcp.json"), wanted)?,
        settings(&root.join(".claude").join("settings.json"), wanted)?,
    ];
    Ok(done.into_iter().flatten().collect())
}

/// Writes what Claude Code needs to work through Kito in the repository at `root`.
pub fn setup(root: &Path) -> Result<Vec<Done>, String> {
    all(root, true)
}

/// Takes out what `setup` (or `kito claude`) wrote, and nothing else.
pub fn remove(root: &Path) -> Result<Vec<Done>, String> {
    all(root, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules_are_kito_s_own_and_come_out_whole() {
        assert!(
            RULES.starts_with(HEADING)
                && RULES.contains("Work in a lane.")
                && RULES.ends_with('\n')
        );
        let own = "# Shop\n\nRun `npm test` before you say it works.\n";
        assert_eq!(
            without_rules(&format!("{}\n\n{RULES}", own.trim_end())),
            own
        );
        assert_eq!(without_rules(RULES), "");
        assert_eq!(
            without_rules(&format!("{RULES}\n## Deploying\n\nAsk first.\n")),
            "## Deploying\n\nAsk first.\n"
        );
        assert_eq!(without_rules(own), own);
        let marked = format!("{own}\n<!-- kito: begin (written by Kito) -->\n\n{RULES}\n# Style\n\nTabs.\n\n<!-- kito: end -->\n");
        assert_eq!(without_rules(&marked), own);
    }
}
