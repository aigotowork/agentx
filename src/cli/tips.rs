//! One-time tips shown on first use of commands.
//!
//! Uses kv store to track which tips have been shown per instance.

#[cfg(test)]
use crate::db::AgentxDb;

pub struct LaunchTipsContext<'a> {
    pub launched: usize,
    pub launcher_name: Option<&'a str>,
    pub launcher_participating: bool,
    pub background: bool,
    pub terminal_mode: &'a str,
    pub terminal_auto_detected: bool,
}

pub use crate::messaging::tips::get_tip;

/// Record the one-time tip only after the originating host flushes its output.
pub(crate) fn maybe_show_tip_client(client: &crate::service::client::Client, instance_name: &str, command: &str, json_output: bool) -> crate::service::ServiceResult<()> {
    use crate::service::{Request, Response, ServiceError, maintenance};
    use std::io::Write;
    if json_output || instance_name.is_empty() {
        return Ok(());
    }
    let Some(text) = get_tip(command) else {
        return Ok(());
    };
    match client.call(Request::Maintenance(maintenance::Request::TipSeen { participant: instance_name.into(), tip: command.into() }))? {
        Response::Maintenance(maintenance::Response::TipSeen(true)) => return Ok(()),
        Response::Maintenance(maintenance::Response::TipSeen(false)) => {}
        _ => return Err(ServiceError::Internal("daemon returned a different tip response".into())),
    }
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "\n{text}").and_then(|_| stdout.flush()).map_err(|error| ServiceError::Unavailable(format!("tip output failed: {error}")))?;
    drop(stdout);
    match client.call(Request::Maintenance(maintenance::Request::MarkTipSeen { participant: instance_name.into(), tip: command.into() }))? {
        Response::Maintenance(maintenance::Response::Updated) => Ok(()),
        _ => Err(ServiceError::Internal("daemon returned a different tip response".into())),
    }
}

/// Check if instance has seen this tip before.
#[cfg(test)]
pub fn has_seen_tip(db: &AgentxDb, instance_name: &str, command: &str) -> bool {
    if instance_name.is_empty() {
        return true;
    }
    let key = format!("tip:{instance_name}:{command}");
    db.kv_get(&key).ok().flatten().is_some()
}

/// Mark tip as seen for this instance.
#[cfg(test)]
pub fn mark_tip_seen(db: &AgentxDb, instance_name: &str, command: &str) {
    if instance_name.is_empty() {
        return;
    }
    let key = format!("tip:{instance_name}:{command}");
    let _ = db.kv_set(&key, Some("1"));
}

/// Show one-time tip for command if not seen before.
#[cfg(test)]
pub fn maybe_show_tip(db: &AgentxDb, instance_name: &str, command: &str, json_output: bool) {
    if json_output {
        return;
    }
    let text = match get_tip(command) {
        Some(t) => t,
        None => return,
    };
    if has_seen_tip(db, instance_name, command) {
        return;
    }
    mark_tip_seen(db, instance_name, command);
    println!("\n{text}");
}

/// Print contextual launch tips; mark one-time tips after exact output flush.
pub(crate) fn print_launch_tips_client(client: &crate::service::client::Client, ctx: LaunchTipsContext<'_>) -> crate::service::ServiceResult<()> {
    use crate::service::{Request, Response, ServiceError, maintenance};
    use std::io::Write;
    let invocation = client.invocation();
    let inside = crate::runtime::context::AgentxContext::from_env(&invocation.environment, invocation.cwd.clone()).is_inside_ai_tool();
    let candidates = launch_tip_candidates(&ctx, inside, &client.scope_root().join("config.toml"));
    let mut lines = Vec::new();
    let mut confirmations = Vec::new();
    for (key, text) in candidates {
        if let (Some(id), Some(key)) = (ctx.launcher_name, key) {
            match client.call(Request::Maintenance(maintenance::Request::TipSeen { participant: id.into(), tip: key.into() }))? {
                Response::Maintenance(maintenance::Response::TipSeen(true)) => continue,
                Response::Maintenance(maintenance::Response::TipSeen(false)) => {}
                _ => return Err(ServiceError::Internal("unexpected launch tip response".into())),
            }
            confirmations.push((id.to_string(), key.to_string()));
        }
        lines.push(text);
    }
    if lines.is_empty() {
        return Ok(());
    }
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "\n{}", lines.join("\n")).and_then(|_| stdout.flush()).map_err(|error| ServiceError::Unavailable(format!("launch tip output failed: {error}")))?;
    drop(stdout);
    for (participant, tip) in confirmations {
        match client.call(Request::Maintenance(maintenance::Request::MarkTipSeen { participant, tip }))? {
            Response::Maintenance(maintenance::Response::Updated) => {}
            _ => return Err(ServiceError::Internal("unexpected launch tip confirmation response".into())),
        }
    }
    Ok(())
}

fn launch_tip_candidates(ctx: &LaunchTipsContext<'_>, inside_tool: bool, config_path: &std::path::Path) -> Vec<(Option<&'static str>, String)> {
    if ctx.launched == 0 {
        return Vec::new();
    }
    let mut tips = Vec::new();
    fn once(tips: &mut Vec<(Option<&'static str>, String)>, key: &'static str, text: &str) {
        tips.push((Some(key), text.to_string()));
    }
    let has_close = ctx.terminal_mode == "kitty"
        || ctx.terminal_mode.starts_with("kitty-")
        || ctx.terminal_mode == "wezterm"
        || ctx.terminal_mode.starts_with("wezterm-")
        || ctx.terminal_mode.starts_with("tmux")
        || ctx.terminal_mode.starts_with("zellij")
        || ctx.terminal_mode == "waveterm"
        || crate::config::load_toml_presets(config_path).and_then(|presets| presets.get(ctx.terminal_mode).and_then(|preset| preset.get("close")).map(|_| true)).unwrap_or(false);
    let managed = if has_close { "managed" } else { "unmanaged" };
    let auto = if ctx.terminal_auto_detected { ", auto-detected" } else { "" };
    tips.push((None, format!("[info] Terminal: {} ({managed}{auto})", ctx.terminal_mode)));
    if inside_tool && ctx.launcher_participating {
        once(&mut tips, "launch:notify", "[tip] You'll be automatically notified when instances are launched & ready");
    }
    if inside_tool {
        if !ctx.launcher_participating {
            once(&mut tips, "launch:start", "[tip] Run 'agentx start' to receive notifications/messages from instances");
        }
        if has_close {
            once(&mut tips, "launch:kill", "[tip] Kill agents and close their panes: agentx kill <name1> <name2> ...");
        }
        if !ctx.background {
            once(&mut tips, "launch:term", "[tip] View an agent's screen: agentx term <name> | Inject keystrokes: agentx term inject <name> [text] --enter");
        }
        if ctx.terminal_mode.starts_with("tmux") || ctx.background {
            once(&mut tips, "launch:sub-blocked", "[tip] Get notified when an agent needs approval: agentx events sub --blocked <name>");
        } else {
            once(&mut tips, "launch:sub-idle", "[tip] Get notified when an agent goes idle: agentx events sub --idle <name>");
        }
        once(&mut tips, "list:status", get_tip("list:status").unwrap_or(""));
    } else {
        once(&mut tips, "launch:send", "[tip] Send a message to an agent: agentx send @<name> <message>");
        once(&mut tips, "launch:list", "[tip] Check status: agentx list");
    }
    tips
}
