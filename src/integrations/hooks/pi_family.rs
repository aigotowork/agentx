//! Hook RPC dispatch shared by Pi, its fork Oh My Pi, and Hermes. Their plugins
//! call `agentx <tool>-{start,status,read,beforetool,stop}` with the same argv
//! shape; only the tool name and OMP's soft-stop recovery differ.

#[cfg(test)]
use crate::integrations::hooks::plugin_rpc::{self, parse_flag};
use crate::integrations::tool::Tool;
#[cfg(test)]
use crate::{
    agents::{binding as instance_binding, lifecycle, records as instances},
    db::AgentxDb,
};
#[cfg(test)]
use crate::{
    log::{log_error, log_info},
    runtime::{bootstrap, context::AgentxContext},
    shared::ST_LISTENING,
};

pub(crate) struct PiFamily {
    pub tool: Tool,
    /// OMP soft-stops on session_shutdown (task subagents share the parent
    /// process), which clears the process binding; the next start recovers it
    /// from `AGENTX_INSTANCE_NAME`.
    pub recover_soft_stopped: bool,
}

pub(crate) const PI: PiFamily = PiFamily { tool: Tool::Pi, recover_soft_stopped: false };

pub(crate) const OMP: PiFamily = PiFamily { tool: Tool::Omp, recover_soft_stopped: true };

pub(crate) const HERMES: PiFamily = PiFamily { tool: Tool::Hermes, recover_soft_stopped: false };

impl PiFamily {
    fn name(&self) -> &'static str {
        self.tool.as_str()
    }

    /// Register the extension's notify port and wake the PTY loop: the bind is
    /// launch readiness for plugin-driven tools (see `delivery.rs`).
    #[cfg(test)]
    pub(crate) fn upsert_plugin_notify_endpoint(&self, db: &AgentxDb, instance_name: &str, port: u16) {
        if let Err(e) = db.upsert_notify_endpoint(instance_name, "plugin", port) {
            log_error("native", &format!("{}.register_notify_fail", self.name()), &format!("Failed to register plugin notify port for {}: {}", instance_name, e));
            return;
        }
        crate::runtime::notify::wake(db, instance_name, crate::runtime::notify::WakeKind::DELIVERY_LOOPS);
    }

    #[cfg(test)]
    fn recover_binding(&self, ctx: &AgentxContext, session_id: &str, process_id: &str, db: &AgentxDb) -> Option<String> {
        if !self.recover_soft_stopped {
            return None;
        }
        let name = ctx.raw_env.get("AGENTX_INSTANCE_NAME").filter(|s| !s.is_empty())?;
        instance_binding::recover_process_binding_for_instance(db, name, session_id, process_id)
    }

    #[cfg(test)]
    pub(crate) fn handle_start(&self, ctx: &AgentxContext, db: &AgentxDb, argv: &[String]) -> (i32, String) {
        // Plugin RPC returns JSON errors on exit 0 so the extension can handle
        // setup failures without the agent treating the hook itself as failed.
        let session_id = match parse_flag(argv, "--session-id") {
            Some(sid) => sid,
            None => return (0, r#"{"error":"Missing --session-id"}"#.to_string()),
        };
        let transcript_path = parse_flag(argv, "--transcript-path");
        let cwd = parse_flag(argv, "--cwd");
        let notify_port: Option<u16> = parse_flag(argv, "--notify-port").and_then(|s| s.parse().ok());

        let process_id = match &ctx.process_id {
            Some(pid) => pid.clone(),
            None => return (0, r#"{"error":"AGENTX_PROCESS_ID not set"}"#.to_string()),
        };

        let Some(instance_name) = instance_binding::bind_session_to_process(db, &session_id, Some(&process_id)).or_else(|| self.recover_binding(ctx, &session_id, &process_id, db)) else {
            return (0, r#"{"error":"No instance bound to this process"}"#.to_string());
        };

        plugin_rpc::initialize_last_event_id(db, &instance_name);
        lifecycle::set_status(db, &instance_name, ST_LISTENING, "start", Default::default());
        instance_binding::capture_and_store_launch_context(db, &instance_name);

        let mut updates = serde_json::Map::new();
        updates.insert("tool".into(), serde_json::json!(self.name()));
        updates.insert("session_id".into(), serde_json::json!(&session_id));
        if let Some(path) = transcript_path.as_ref().filter(|p| !p.is_empty()) {
            updates.insert("transcript_path".into(), serde_json::json!(path));
        }
        let cwd_value = cwd.as_deref().filter(|p| !p.is_empty()).or_else(|| ctx.cwd.to_str());
        if let Some(cwd) = cwd_value {
            updates.insert("directory".into(), serde_json::json!(cwd));
        }
        instances::update_instance_position(db, &instance_name, &updates);
        if let Some(port) = notify_port {
            self.upsert_plugin_notify_endpoint(db, &instance_name, port);
        }
        log_info("hooks", &format!("{}-start.bind", self.name()), &format!("instance={} session_id={}", instance_name, session_id));
        crate::relay::worker::try_spawn_worker();

        let response = serde_json::json!({
            "name": instance_name,
            "session_id": session_id,
            "bootstrap": bootstrap::get_bootstrap(db, ctx, &instance_name, self.name()),
        });
        (0, response.to_string())
    }

    pub(crate) fn dispatch(&self, hook_name: &str, argv: &[String]) -> (i32, String) {
        super::plugin_client::dispatch(self.name(), hook_name, argv, self.recover_soft_stopped)
    }
}
