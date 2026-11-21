//! `kito-claude`: what it writes for Claude Code in a project.
//!
//! The last test runs it under Kito itself, next to `kito claude`, and
//! needs `kito` and `kitod` in `KITO_BIN` or in `../kito/target/debug`
//! (`cargo build -p kito-cli -p kitod` there).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

fn plugin(root: &Path, args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_kito-claude"))
        .args(args)
        .env("KITO_REPO", root)
        .output()
        .unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), said.replace('\\', "/"))
}

fn text(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn json_of(path: PathBuf) -> Value {
    serde_json::from_str(&text(path)).unwrap()
}

#[test]
fn it_writes_what_claude_code_reads_and_takes_it_out_again() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let (ok, said) = plugin(root, &["assistant", "setup"]);
    assert!(ok, "{said}");
    for file in [
        "wrote CLAUDE.md",
        "wrote .mcp.json",
        "wrote .claude/settings.json",
    ] {
        assert!(said.contains(file), "{said}");
    }

    let rules = text(root.join("CLAUDE.md"));
    assert!(
        rules.starts_with("## Version control: Kito\n") && rules.contains("Work in a lane."),
        "{rules}"
    );
    assert_eq!(
        json_of(root.join(".mcp.json")),
        json!({ "mcpServers": { "kito": { "command": "kito-mcp" } } })
    );
    let settings = json_of(root.join(".claude/settings.json"));
    assert_eq!(
        settings["permissions"],
        json!({ "allow": ["mcp__kito__*"] })
    );
    assert_eq!(
        settings["hooks"],
        json!({
            "PreToolUse": [{ "matcher": "Edit|Write|MultiEdit|NotebookEdit", "hooks": [
                { "type": "command", "command": "kito hook before-edit", "timeout": 30 },
            ]}],
            "PostToolUse": [{ "matcher": "Bash", "hooks": [
                { "type": "command", "command": "kito hook after-shell", "timeout": 30 },
            ]}],
            "SessionStart": [{ "hooks": [
                { "type": "command", "command": "kito hook session-start", "timeout": 30 },
            ]}],
        })
    );

    // Again: nothing to do. Out again: nothing left.
    let (ok, said) = plugin(root, &["assistant", "setup"]);
    assert!(
        ok && said.contains("everything was there already"),
        "{said}"
    );
    let (ok, said) = plugin(root, &["assistant", "remove"]);
    assert!(ok && said.contains("removed CLAUDE.md"), "{said}");
    assert!(
        !root.join("CLAUDE.md").exists()
            && !root.join(".mcp.json").exists()
            && !root.join(".claude").exists()
    );
    let (ok, said) = plugin(root, &["assistant", "remove"]);
    assert!(ok && said.contains("nothing of it was there"), "{said}");
}

#[test]
fn what_the_project_says_itself_is_left_as_it_is() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let own_rules = "# Shop\n\nRun `npm test` before you say it works.\n";
    let own_servers =
        json!({ "mcpServers": { "docs": { "command": "docs-mcp", "args": ["--stdio"] } } });
    let own_settings = json!({
        "model": "opus",
        "permissions": { "allow": ["Bash(npm test)"], "deny": ["Read(.env)"] },
        "hooks": { "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "./check.sh" }] }] },
    });
    std::fs::create_dir_all(root.join(".claude")).unwrap();
    std::fs::write(root.join("CLAUDE.md"), own_rules).unwrap();
    std::fs::write(root.join(".mcp.json"), own_servers.to_string()).unwrap();
    std::fs::write(root.join(".claude/settings.json"), own_settings.to_string()).unwrap();

    let (ok, said) = plugin(root, &["assistant", "setup"]);
    assert!(ok && said.contains("changed CLAUDE.md"), "{said}");
    let rules = text(root.join("CLAUDE.md"));
    assert!(
        rules.starts_with(own_rules) && rules.contains("\n\n## Version control: Kito\n"),
        "{rules}"
    );
    let servers = json_of(root.join(".mcp.json"));
    assert_eq!(
        servers["mcpServers"]["docs"],
        own_servers["mcpServers"]["docs"]
    );
    assert_eq!(servers["mcpServers"]["kito"]["command"], "kito-mcp");
    let settings = json_of(root.join(".claude/settings.json"));
    assert_eq!(settings["model"], "opus");
    assert_eq!(
        settings["permissions"],
        json!({ "allow": ["Bash(npm test)", "mcp__kito__*"], "deny": ["Read(.env)"] })
    );
    let before = settings["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(
        (before.len(), before[0]["hooks"][0]["command"].as_str()),
        (2, Some("./check.sh"))
    );

    // Taken out, the project has what it had.
    let (ok, said) = plugin(root, &["assistant", "remove"]);
    assert!(ok, "{said}");
    assert_eq!(text(root.join("CLAUDE.md")), own_rules);
    assert_eq!(json_of(root.join(".mcp.json")), own_servers);
    assert_eq!(json_of(root.join(".claude/settings.json")), own_settings);

    // Settings that say the same are not written again for their looks.
    let compact = json!({ "mcpServers": { "kito": { "command": "kito-mcp" } } }).to_string();
    std::fs::write(root.join(".mcp.json"), &compact).unwrap();
    plugin(root, &["assistant", "setup"]);
    assert_eq!(text(root.join(".mcp.json")), compact);

    // A server called `kito` that is the project's own is neither replaced nor taken out.
    let another = json!({ "mcpServers": { "kito": { "command": "another" } } });
    std::fs::write(root.join(".mcp.json"), another.to_string()).unwrap();
    plugin(root, &["assistant", "setup"]);
    plugin(root, &["assistant", "remove"]);
    assert_eq!(json_of(root.join(".mcp.json")), another);

    // What cannot be read is not touched, and said.
    std::fs::write(root.join(".claude/settings.json"), "not json").unwrap();
    let (ok, said) = plugin(root, &["assistant", "setup"]);
    assert!(
        !ok && said.contains("is not JSON") && said.contains("by hand"),
        "{said}"
    );
    assert_eq!(text(root.join(".claude/settings.json")), "not json");
}

#[test]
fn outside_kito_it_says_how_it_is_run() {
    let out = Command::new(env!("CARGO_BIN_EXE_kito-claude"))
        .args(["assistant", "setup"])
        .env_remove("KITO_REPO")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("kito plugin enable claude"));
}

fn kito_bin() -> PathBuf {
    let dir = std::env::var_os("KITO_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../kito/target/debug"));
    let kito = dir.join(format!("kito{}", std::env::consts::EXE_SUFFIX));
    assert!(
        kito.is_file(),
        "{} is not there: this test runs kito-claude under Kito. Build it (`cargo build -p kito-cli -p kitod` in \
         the kito repository) or set KITO_BIN to where `kito` and `kitod` are",
        kito.display()
    );
    dir.canonicalize().unwrap()
}

#[test]
fn it_writes_what_kito_itself_writes() {
    let bin = kito_bin();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let (root, config) = (base.join("shop"), base.join("config"));
    std::fs::create_dir_all(&root).unwrap();
    // `with_claude`: Kito sets Claude Code up itself, as it does by default.
    let kito = |args: &[&str], with_claude: bool| -> String {
        let mut command = Command::new(bin.join(format!("kito{}", std::env::consts::EXE_SUFFIX)));
        command
            .args(args)
            .current_dir(&root)
            .env(
                "KITOD_BIN",
                bin.join(format!("kitod{}", std::env::consts::EXE_SUFFIX)),
            )
            .env("KITO_AUTHOR", "ada")
            .env("KITO_CONFIG_DIR", &config);
        match with_claude {
            true => command.env_remove("KITO_NO_CLAUDE"),
            false => command.env("KITO_NO_CLAUDE", "1"),
        };
        let out = command.output().unwrap();
        let (said, errors) = (
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        assert!(out.status.success(), "kito {args:?}: {said}{errors}");
        said.into_owned()
    };
    kito(&["init"], false);
    std::fs::write(root.join("shop.ts"), "export const price = 1;\n").unwrap();
    assert!(!root.join("CLAUDE.md").exists());

    // The plugin as it is installed from: the program and its manifest.
    let source = base.join("plugin");
    std::fs::create_dir_all(&source).unwrap();
    let program = Path::new(env!("CARGO_BIN_EXE_kito-claude"));
    std::fs::copy(program, source.join(program.file_name().unwrap())).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("kito-plugin.toml"),
        source.join("kito-plugin.toml"),
    )
    .unwrap();
    kito(
        &["plugin", "add", "--path", source.to_str().unwrap()],
        false,
    );
    kito(&["plugin", "enable", "claude"], false);
    assert!(kito(&["assistant", "list"], false).contains("claude  (plugin claude"));
    let set_up = kito(&["assistant", "add", "claude"], false);
    assert!(
        set_up.contains("Claude Code is set up for Kito") && set_up.contains("wrote CLAUDE.md"),
        "{set_up}"
    );

    // Kito itself, asked to set Claude Code up, finds nothing to add: the
    // same rules, the same server, the same permission, the same hooks.
    let files = [
        root.join("CLAUDE.md"),
        root.join(".mcp.json"),
        root.join(".claude/settings.json"),
    ];
    let by_plugin: Vec<String> = files.iter().map(|file| text(file.clone())).collect();
    kito(&["claude"], true);
    let by_kito: Vec<String> = files.iter().map(|file| text(file.clone())).collect();
    assert_eq!(by_plugin[0], by_kito[0], "the rules");
    for (plugin, kito) in by_plugin.iter().zip(&by_kito).skip(1) {
        assert_eq!(
            serde_json::from_str::<Value>(plugin).unwrap(),
            serde_json::from_str::<Value>(kito).unwrap()
        );
    }

    // And what Kito wrote, the plugin takes out.
    let removed = kito(&["assistant", "remove", "claude"], false);
    assert!(removed.contains("removed CLAUDE.md"), "{removed}");
    assert!(files.iter().all(|file| !file.exists()));
    kito(&["claude"], true);
    assert!(files.iter().all(|file| file.is_file()));
    kito(&["assistant", "remove", "claude"], false);
    assert!(files.iter().all(|file| !file.exists()));

    // Disabled, it takes out what it wrote without being asked to.
    kito(&["assistant", "add", "claude"], false);
    assert!(files.iter().all(|file| file.is_file()));
    let disabled = kito(&["plugin", "disable", "claude"], false);
    assert!(
        disabled.contains("removed CLAUDE.md") && disabled.contains("claude is disabled"),
        "{disabled}"
    );
    assert!(files.iter().all(|file| !file.exists()));
    kito(&["daemon", "stop"], false);
}
