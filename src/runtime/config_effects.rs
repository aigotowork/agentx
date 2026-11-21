//! Host-owned configuration application. File edits belong to config::file;
//! Provider permission refresh happens here, outside daemon dispatch.
use crate::service::InvocationContext;

/// Per-run adapters receive the explicit invocation. Existing persistent hook
/// installers use their real host process paths, so a different environment
/// needs a separately scoped host rather than process-global env replacement.
pub(crate) fn refresh_installed_hook_permissions(invocation: &InvocationContext, enabled: bool) -> Vec<(&'static str, String)> {
    let mut failures = Vec::new();
    let host_environment: std::collections::HashMap<String, String> = std::env::vars().collect();
    for spec in crate::integrations::registry::ALL.iter().filter(|spec| spec.released) {
        let tool = spec.tool;
        if let Some(adapter) = crate::integrations::hooks::runtime::adapter(tool) {
            let context = crate::integrations::hooks::runtime::LaunchCtx { tool, env: invocation.environment.clone(), cwd: invocation.cwd.clone(), args: Vec::new(), auto_approve: enabled };
            if let Some(ensure) = adapter.ensure_permissions
                && let Err(error) = ensure(&context)
            {
                failures.push((tool.as_str(), format!("{error:#}")));
            }
            continue;
        }
        if invocation.environment != host_environment {
            failures.push((tool.as_str(), "persistent hook permission refresh requires the originating host environment".into()));
            continue;
        }
        if !tool.verify_hooks_installed(false) {
            continue;
        }
        if let Err(error) = tool.try_setup_hooks(enabled) {
            failures.push((tool.as_str(), error));
        }
    }
    failures
}

pub(crate) fn apply_for_scope(scope: &std::path::Path, invocation: &InvocationContext, key: &str) -> Result<(), String> {
    if !crate::config::user_field(key).is_some_and(|field| field.name == "auto_approve") {
        return Ok(());
    }
    // Shared permission files follow config.toml, not a per-invocation override.
    let enabled = crate::config::AgentxConfig::load_for_scope(scope, &Default::default()).auto_approve;
    let failures = refresh_installed_hook_permissions(invocation, enabled);
    if failures.is_empty() { Ok(()) } else { Err(failures.into_iter().map(|(tool, error)| format!("{tool}: {error}")).collect::<Vec<_>>().join("; ")) }
}
