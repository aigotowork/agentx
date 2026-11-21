//! Workspace selection for an already resolved caller. Persisted membership
//! wins over mutable process environment; explicit operation selectors still win.
use crate::service::queries::{AgentFilter, AgentWorkspace};
use std::path::PathBuf;

/// Shared input grammar for team views and Agent pickers.
#[derive(clap::Args, Debug, Default, PartialEq, Eq)]
pub(crate) struct AgentFilterArgs {
    /// Discover the workspace containing this directory.
    #[arg(long, value_name = "PATH")]
    pub dir: Option<String>,
    /// Select an exact workspace directory, or global.
    #[arg(long, value_name = "PATH|global")]
    pub workspace: Option<String>,
    /// Filter current local Agents by key=value, key!=value, key or !key; repeat to AND conditions.
    #[arg(long = "label", value_name = "CONDITION")]
    pub labels: Vec<String>,
}

impl AgentFilterArgs {
    pub(crate) fn has_workspace_selector(&self) -> bool {
        self.dir.is_some() || self.workspace.is_some()
    }

    pub(crate) fn workspace(&self, session: &super::client::Session, default_team: bool) -> anyhow::Result<AgentWorkspace> {
        if !default_team && !self.has_workspace_selector() {
            return Ok(AgentWorkspace::All);
        }
        let path = selected(session, self.dir.as_deref(), self.workspace.as_deref())?;
        Ok(match path {
            Some(path) => AgentWorkspace::Team(path.to_string_lossy().into_owned()),
            None => AgentWorkspace::Global,
        })
    }

    pub(crate) fn resolve(&self, session: &super::client::Session, default_team: bool) -> anyhow::Result<AgentFilter> {
        Ok(AgentFilter { workspace: self.workspace(session, default_team)?, labels: self.label_predicates()? })
    }

    pub(crate) fn label_predicates(&self) -> crate::service::ServiceResult<Vec<crate::agents::labels::LabelPredicate>> {
        let labels = self.labels.iter().map(|raw| crate::agents::labels::parse_selector(raw)).collect::<crate::service::ServiceResult<Vec<_>>>()?;
        crate::agents::labels::normalize_selectors(&labels)
    }
}

#[derive(clap::Parser, Debug)]
#[command(name = "agentx")]
pub(crate) struct DashboardArgs {
    #[command(flatten)]
    pub filters: AgentFilterArgs,
    /// Include every workspace in this AgentX scope (the default).
    #[arg(long, conflicts_with_all = ["dir", "workspace"])]
    pub all: bool,
    /// Open the dashboard in another terminal.
    #[arg(long)]
    pub new_terminal: bool,
}

pub(crate) fn starts_dashboard(option: &str) -> bool {
    matches!(option.split('=').next(), Some("--dir" | "--workspace" | "--label" | "--all"))
}

pub(crate) fn selected(session: &super::client::Session, dir: Option<&str>, workspace: Option<&str>) -> anyhow::Result<Option<PathBuf>> {
    let mut environment = session.invocation.environment.clone();
    if let Some(actor) = &session.actor {
        let membership = crate::agents::workspaces::workspace(&actor.record);
        environment.insert("AGENTX_WORKSPACE".into(), membership.unwrap_or_else(|| "global".into()));
    }
    crate::config::roles::select_workspace(&session.invocation.cwd, &environment, dir, workspace)
}

pub(crate) fn visible(record: &crate::db::InstanceRow, workspace: Option<&str>) -> bool {
    let membership = crate::agents::workspaces::workspace(record);
    AgentWorkspace::visible_in_team(membership.as_deref(), workspace)
}
