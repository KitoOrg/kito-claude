//! `kito-claude`: Claude Code set up to work through Kito.
//!
//! A plugin of Kito (ADR 0005 there). In a repository that enabled it:
//!
//! - `kito assistant add claude` (which runs `kito-claude assistant
//!   setup`) writes what Claude Code reads in a project: the rules in
//!   `CLAUDE.md`, Kito's tools in `.mcp.json`, and the permission to use
//!   them and the hooks in `.claude/settings.json`;
//! - `kito assistant remove claude` takes exactly that out again.
//!
//! It writes what Kito itself writes (`kito claude`), in the same words:
//! a project set up by one is set up for the other. The hooks are Kito's
//! own commands (`kito hook …`), which read what Claude Code says.

mod setup;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The repository, as Kito says it to its plugins.
const ENV_REPO: &str = "KITO_REPO";

fn repository() -> Result<PathBuf, String> {
    std::env::var_os(ENV_REPO)
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .filter(|root| root.is_dir())
        .ok_or_else(|| {
            "this is a plugin of Kito: `kito plugin enable claude` in a repository, then `kito assistant add claude`"
                .to_owned()
        })
}

fn said(root: &Path, done: Vec<setup::Done>, nothing: &str) {
    if done.is_empty() {
        println!("{nothing}");
    }
    for one in done {
        let path = one.path.strip_prefix(root).unwrap_or(&one.path);
        println!("  {} {}", one.what, path.display());
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["assistant", "setup"] => repository().and_then(|root| {
            let done = setup::setup(&root)?;
            println!("Claude Code is set up for Kito in this repository:");
            said(&root, done, "  everything was there already");
            println!("Claude Code asks once whether to use the server of `.mcp.json`, when it starts here.");
            println!("`kito` and `kito-mcp` must be on the PATH Claude Code runs with.");
            Ok(())
        }),
        ["assistant", "remove"] => repository().and_then(|root| {
            let done = setup::remove(&root)?;
            println!("Claude Code is no longer set up for Kito in this repository:");
            said(&root, done, "  nothing of it was there");
            Ok(())
        }),
        // One last time, where the plugin is disabled (`[cleanup]`, Kito
        // 0.23): what it wrote in the project comes out with it.
        ["cleanup", "repository"] => repository().and_then(|root| {
            for one in setup::remove(&root)? {
                println!("{} {}", one.what, one.path.strip_prefix(&root).unwrap_or(&one.path).display());
            }
            Ok(())
        }),
        ["cleanup", _] => Ok(()),
        ["--version"] => {
            println!("kito-claude {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err("it is `kito assistant add claude` or `kito assistant remove claude`".to_owned()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("kito-claude: {why}");
            ExitCode::from(1)
        }
    }
}
