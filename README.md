# kito-claude

[Claude Code](https://claude.com/claude-code) set up to work through
[Kito](https://github.com/KitoOrg/kito), as a plugin of Kito.

```console
$ kito plugin add --path plugin
$ cd your-repository
$ kito plugin enable claude
$ kito assistant add claude
Claude Code is set up for Kito in this repository:
  wrote CLAUDE.md
  wrote .mcp.json
  wrote .claude/settings.json
```

`kito assistant remove claude` takes exactly that out again.

## What it writes

- **`CLAUDE.md`**: the rules of a project kept by Kito (work in a lane,
  change what the lane declared, know why code is the way it is), after
  whatever the project says there itself.
- **`.mcp.json`**: Kito's tools, the server `kito` running `kito-mcp`.
- **`.claude/settings.json`**: the permission to use those tools
  (`mcp__kito__*`), and what Claude Code runs whatever the model decided:
  `kito hook before-edit` before an edit, `kito hook after-shell` after a
  shell command, `kito hook session-start` when a session starts.

What is there already stays: other servers, other permissions, other
hooks, the project's own instructions. A server called `kito` that is
not Kito's is neither replaced nor taken out.

## Next to `kito claude`

Kito sets Claude Code up itself, when a repository is made and with
`kito claude`. This plugin writes the same things in the same words (the
rules are Kito's own text, taken from it when the plugin is built, and a
test holds the two together): a project set up by one is set up for the
other. What the plugin adds is the way out, `kito assistant remove
claude`, and that Claude Code is then one assistant among the others
(`kito assistant list`).

Rules that an older Kito wrote are brought up to date by `kito claude`,
which knows them; the plugin only sees that rules are there.

`kito` and `kito-mcp` must be on the PATH Claude Code runs with.

## Development

```console
$ cargo test
```

The last test wants `kito` and `kitod` in `KITO_BIN` or in
`../kito/target/debug`. `rules.md` is Kito's `RULES` (`crates/kito-cli/
src/claude.rs`): when Kito changes them, it is copied again.

## License

MIT or Apache-2.0, at your option.
