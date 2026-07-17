//! `meleys setup` — register Meleys as the browser backend for coding agents.
//!
//! On install, the WiX MSI runs `meleys setup install`, which writes a stdio
//! MCP-server entry for Meleys into the config files of supported coding agents
//! (Claude Code, Cline, Cursor, VS Code/Copilot) and disables their built-in
//! `WebSearch`/`WebFetch` tools so the agent routes web access through Meleys.
//!
//! All edits are idempotent and reversible: every key Meleys adds is marked with
//! `"_meleys_managed": true`, and `setup uninstall` removes only those keys, never
//! user-authored servers or settings.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

const SERVER_NAME: &str = "meleys";
const MANAGED_MARKER: &str = "_meleys_managed";

/// How a target agent stores its MCP servers.
enum ConfigKind {
    /// `mcpServers` object at the JSON root. Used by Cline (`~/.cline/mcp.json`),
    /// Cursor (`~/.cursor/mcp.json`), and project `.mcp.json`.
    McpServersRoot,
    /// Claude Code user config (`~/.claude.json`): `mcpServers` object at root,
    /// but a separate `settings.json` controls `permissions.deny`.
    ClaudeJson,
    /// VS Code / GitHub Copilot: servers live under `mcp.servers.<name>` in
    /// `settings.json` (different nest from `mcpServers`).
    VSCodeSettings,
}

struct Agent {
    id: &'static str,
    display: &'static str,
    /// Relative path from the user's home dir (or, for VS Code, from config dir).
    config_rel: &'static str,
    kind: ConfigKind,
    /// Whether built-in web tools should be disabled for this agent.
    disable_builtin: bool,
}

fn agents() -> Vec<Agent> {
    vec![
        Agent {
            id: "claude",
            display: "Claude Code",
            config_rel: ".claude.json",
            kind: ConfigKind::ClaudeJson,
            disable_builtin: true,
        },
        Agent {
            id: "cline",
            display: "Cline",
            config_rel: ".cline/mcp.json",
            kind: ConfigKind::McpServersRoot,
            disable_builtin: false,
        },
        Agent {
            id: "cursor",
            display: "Cursor",
            config_rel: ".cursor/mcp.json",
            kind: ConfigKind::McpServersRoot,
            disable_builtin: false,
        },
        Agent {
            id: "vscode",
            display: "VS Code / GitHub Copilot",
            // VS Code user settings live under %APPDATA%/Code/User on Windows.
            config_rel: "Code/User/settings.json",
            kind: ConfigKind::VSCodeSettings,
            disable_builtin: false,
        },
    ]
}

/// Resolve the absolute config path for an agent. VS Code uses the OS config dir;
/// the others use the user's home dir.
fn config_path(agent: &Agent) -> Option<PathBuf> {
    if matches!(agent.kind, ConfigKind::VSCodeSettings) {
        dirs::config_dir().map(|d| d.join(agent.config_rel))
    } else {
        dirs::home_dir().map(|d| d.join(agent.config_rel))
    }
}

/// The MCP-server entry we register for Meleys. Uses the absolute path of the
/// running executable so agents can spawn it regardless of working directory.
fn meleys_entry(exe: &Path) -> Value {
    let exe_str = exe.to_string_lossy().to_string();
    json!({
        "command": exe_str,
        "args": ["--mcp"],
        "env": {},
        MANAGED_MARKER: true,
    })
}

fn vscode_entry(exe: &Path) -> Value {
    let exe_str = exe.to_string_lossy().to_string();
    json!({
        "type": "stdio",
        "command": exe_str,
        "args": ["--mcp"],
    })
}

/// Strip JSONC noise (line/block comments and trailing commas) so we can parse
/// config files that tools like VS Code write in JSONC rather than strict JSON.
fn strip_jsonc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    let mut escape = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(c) = chars.next() {
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
                out.push(c);
            }
            continue;
        }
        if in_block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            in_line_comment = true;
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_block_comment = true;
            continue;
        }
        out.push(c);
    }

    // Remove trailing commas before } or ].
    strip_trailing_commas(&out)
}

/// Remove trailing commas that precede `}` or `]` (single pass, string-aware).
fn strip_trailing_commas(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    let mut escape = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }
        if c == ',' {
            // Look ahead for the next non-whitespace char.
            let mut ahead = chars.clone();
            let mut skipped = 0;
            let mut next = None;
            for nc in &mut ahead {
                if nc.is_whitespace() {
                    skipped += 1;
                    continue;
                }
                next = Some(nc);
                break;
            }
            if next == Some('}') || next == Some(']') {
                // Drop the comma (and the whitespace we would have copied anyway).
                for _ in 0..skipped {
                    chars.next();
                }
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// Read a JSON/JSONC file, returning an empty object if missing or empty.
fn read_json(path: &Path) -> anyhow::Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = std::fs::read_to_string(path)?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    let cleaned = strip_jsonc(&text);
    let v: Value = serde_json::from_str(&cleaned)?;
    Ok(v)
}

fn write_json(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Insert or replace the Meleys server under a `mcpServers` object (root level).
fn upsert_mcp_servers(root: &mut Value, entry: &Value) {
    let servers = root
        .as_object_mut()
        .unwrap()
        .entry("mcpServers")
        .or_insert_with(|| json!({}));
    if let Some(map) = servers.as_object_mut() {
        map.insert(SERVER_NAME.to_string(), entry.clone());
    }
}

/// Remove only the Meleys-managed server from a `mcpServers` object.
fn remove_mcp_servers(root: &mut Value) {
    if let Some(servers) = root.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
        servers.remove(SERVER_NAME);
        if servers.is_empty() {
            if let Some(obj) = root.as_object_mut() {
                obj.remove("mcpServers");
            }
        }
    }
}

/// Add `WebSearch`/`WebFetch` to an agent's `permissions.deny` list (idempotent).
fn add_deny_permissions(root: &mut Value) {
    let obj = root.as_object_mut().unwrap();
    let permissions = obj.entry("permissions").or_insert_with(|| json!({}));
    let perms_obj = permissions.as_object_mut().unwrap();
    let deny = perms_obj.entry("deny").or_insert_with(|| json!([]));
    if let Some(arr) = deny.as_array_mut() {
        for tool in ["WebSearch", "WebFetch"] {
            if !arr.iter().any(|v| v.as_str() == Some(tool)) {
                arr.push(json!(tool));
            }
        }
    }
}

fn remove_deny_permissions(root: &mut Value) {
    if let Some(perms) = root.get_mut("permissions").and_then(|p| p.as_object_mut()) {
        if let Some(arr) = perms.get_mut("deny").and_then(|d| d.as_array_mut()) {
            arr.retain(|v| v.as_str() != Some("WebSearch") && v.as_str() != Some("WebFetch"));
        }
        if perms
            .get("deny")
            .and_then(|d| d.as_array())
            .map(|a| a.is_empty())
            == Some(true)
        {
            perms.remove("deny");
        }
        if perms.is_empty() {
            if let Some(obj) = root.as_object_mut() {
                obj.remove("permissions");
            }
        }
    }
}

/// Install Meleys into one agent's config. Returns a human-readable status line.
fn install_agent(agent: &Agent, exe: &Path) -> anyhow::Result<String> {
    let Some(path) = config_path(agent) else {
        return Ok(format!(
            "{}: skipped (home/config dir not found)",
            agent.display
        ));
    };

    match agent.kind {
        ConfigKind::McpServersRoot => {
            let mut root = read_json(&path)?;
            upsert_mcp_servers(&mut root, &meleys_entry(exe));
            write_json(&path, &root)?;
        }
        ConfigKind::ClaudeJson => {
            // MCP server into ~/.claude.json
            let mut root = read_json(&path)?;
            upsert_mcp_servers(&mut root, &meleys_entry(exe));
            write_json(&path, &root)?;
            // Disable built-ins in ~/.claude/settings.json (user scope) if requested.
            if agent.disable_builtin {
                let settings = dirs::home_dir().map(|h| h.join(".claude").join("settings.json"));
                if let Some(sp) = settings {
                    let mut sroot = read_json(&sp)?;
                    add_deny_permissions(&mut sroot);
                    write_json(&sp, &sroot)?;
                }
            }
        }
        ConfigKind::VSCodeSettings => {
            let mut root = read_json(&path)?;
            let servers = root
                .as_object_mut()
                .unwrap()
                .entry("mcp")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .unwrap()
                .entry("servers")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .unwrap();
            servers.insert(SERVER_NAME.to_string(), vscode_entry(exe));
            write_json(&path, &root)?;
        }
    }

    Ok(format!(
        "{}: configured at {}",
        agent.display,
        path.display()
    ))
}

/// Remove Meleys from one agent's config (only our managed keys).
fn uninstall_agent(agent: &Agent) -> anyhow::Result<String> {
    let Some(path) = config_path(agent) else {
        return Ok(format!(
            "{}: skipped (home/config dir not found)",
            agent.display
        ));
    };
    if !path.exists() {
        return Ok(format!("{}: nothing to remove", agent.display));
    }

    match agent.kind {
        ConfigKind::McpServersRoot => {
            let mut root = read_json(&path)?;
            remove_mcp_servers(&mut root);
            write_json(&path, &root)?;
        }
        ConfigKind::ClaudeJson => {
            let mut root = read_json(&path)?;
            remove_mcp_servers(&mut root);
            write_json(&path, &root)?;
            let settings = dirs::home_dir().map(|h| h.join(".claude").join("settings.json"));
            if let Some(sp) = settings {
                if sp.exists() {
                    let mut sroot = read_json(&sp)?;
                    remove_deny_permissions(&mut sroot);
                    write_json(&sp, &sroot)?;
                }
            }
        }
        ConfigKind::VSCodeSettings => {
            let mut root = read_json(&path)?;
            if let Some(servers) = root
                .get_mut("mcp")
                .and_then(|m| m.get_mut("servers"))
                .and_then(|s| s.as_object_mut())
            {
                servers.remove(SERVER_NAME);
            }
            write_json(&path, &root)?;
        }
    }

    Ok(format!(
        "{}: removed from {}",
        agent.display,
        path.display()
    ))
}

/// Report whether Meleys is currently registered for an agent.
fn status_agent(agent: &Agent) -> String {
    let Some(path) = config_path(agent) else {
        return format!("{}: ? (home/config dir not found)", agent.display);
    };
    let root = match read_json(&path) {
        Ok(v) => v,
        Err(_) => {
            return format!(
                "{}: ? (unreadable config at {})",
                agent.display,
                path.display()
            );
        }
    };
    let ok = match agent.kind {
        ConfigKind::VSCodeSettings => root
            .get("mcp")
            .and_then(|m| m.get("servers"))
            .and_then(|s| s.get(SERVER_NAME))
            .is_some(),
        _ => root
            .get("mcpServers")
            .and_then(|s| s.get(SERVER_NAME))
            .is_some(),
    };
    format!(
        "{}: {}",
        agent.display,
        if ok { "configured" } else { "not configured" }
    )
}

fn resolve_targets(specified: &[String]) -> Vec<Agent> {
    let all = agents();
    if specified.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|a| specified.iter().any(|s| s == a.id))
        .collect()
}

/// Entry point for `meleys setup <subcommand> [--agents a,b,c] [--no-disable-builtin]`.
pub fn run(args: &[String]) -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let exe = if exe.exists() {
        exe
    } else {
        PathBuf::from("meleys")
    };

    let mut subcommand = "install";
    let mut targets: Vec<String> = vec![];
    let mut disable_builtin = true;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "install" | "uninstall" | "list" => subcommand = args[i].as_str(),
            "--agents" => {
                if let Some(next) = args.get(i + 1) {
                    targets = next.split(',').map(|s| s.trim().to_string()).collect();
                    i += 1;
                }
            }
            "--no-disable-builtin" => disable_builtin = false,
            other if other.starts_with("--agents=") => {
                targets = other["--agents=".len()..]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
            _ => {}
        }
        i += 1;
    }

    let mut selected = resolve_targets(&targets);
    if !disable_builtin {
        for a in selected.iter_mut() {
            a.disable_builtin = false;
        }
    }
    if selected.is_empty() {
        anyhow::bail!("No known agents matched. Valid: claude, cline, cursor, vscode");
    }

    match subcommand {
        "install" => {
            println!(
                "Registering Meleys ({}) as browser backend for coding agents...",
                exe.display()
            );
            for agent in &selected {
                match install_agent(agent, &exe) {
                    Ok(msg) => println!("  ✓ {}", msg),
                    Err(e) => eprintln!("  ✗ {} failed: {}", agent.display, e),
                }
            }
            println!("Done. Restart the relevant agent to load the new MCP server.");
        }
        "uninstall" => {
            println!("Removing Meleys browser-backend registration from coding agents...");
            for agent in &selected {
                match uninstall_agent(agent) {
                    Ok(msg) => println!("  ✓ {}", msg),
                    Err(e) => eprintln!("  ✗ {} failed: {}", agent.display, e),
                }
            }
        }
        "list" => {
            for agent in &selected {
                println!("  • {}", status_agent(agent));
            }
        }
        _ => anyhow::bail!("Unknown setup subcommand. Use install, uninstall, or list."),
    }

    Ok(())
}
