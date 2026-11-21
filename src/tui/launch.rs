//! Editable quick-launch choices. Only explicit overrides go into the submitted command.
use super::model::{LaunchField, LaunchState};
use crate::config::{profiles::ProfileCatalog, roles};
use crate::runtime::{
    launch_options::AgentxLaunchFlags,
    launch_profile::{LaunchSelection, prepare_with_role},
};
use crate::service::InvocationContext;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

#[derive(Default)]
pub(super) struct Catalog {
    pub roles: Vec<(String, String)>,
    pub profiles: Vec<(String, String)>,
    pub scope: PathBuf,
    pub cwd: PathBuf,
    pub environment: HashMap<String, String>,
}

impl Catalog {
    fn load() -> anyhow::Result<Self> {
        let invocation = InvocationContext::capture()?;
        let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
        let actor = crate::cli::commands::roles::discovery_actor(&scope, &invocation)?;
        let collaboration = actor.as_ref().and_then(|actor| crate::agents::workspaces::collaboration(&actor.record));
        let mut environment = invocation.environment;
        if let Some(actor) = actor {
            environment.insert("AGENTX_WORKSPACE".into(), crate::agents::workspaces::workspace(&actor.record).unwrap_or_else(|| "global".into()));
        }
        let workspace = roles::select_workspace(&invocation.cwd, &environment, None, None)?;
        let roles = roles::list_roles_for(&scope, workspace.as_deref(), false, collaboration.as_ref().and_then(|c| c.role.as_ref()))?.into_iter().map(|r| (r.name, r.description.unwrap_or(r.namespace))).collect();
        let profiles = ProfileCatalog::load(&scope)?.profiles.into_iter().map(|(name, profile)| (name, profile.tool)).collect();
        Ok(Self { roles, profiles, scope, cwd: invocation.cwd, environment })
    }
}

#[derive(Clone, Default)]
pub struct QuickLaunch {
    pub role: Option<String>,
    pub profile: Option<String>,
    pub advanced: bool,
    pub picker: Option<LaunchField>,
    pub query: String,
    pub index: usize,
    pub profile_label: String,
    pub source_label: String,
    pub error: Option<String>,
    pub(super) catalog: Option<Arc<Catalog>>,
}

impl LaunchState {
    pub(super) fn load_catalog(&mut self) {
        match Catalog::load() {
            Ok(catalog) => {
                self.quick.catalog = Some(Arc::new(catalog));
                self.refresh_preview();
            }
            Err(error) => self.quick.error = Some(format!("{error:#}")),
        }
    }

    pub(super) fn refresh_preview(&mut self) {
        let Some(catalog) = &self.quick.catalog else { return };
        let flags = AgentxLaunchFlags { role: self.quick.role.clone(), ..Default::default() };
        let selection = LaunchSelection { profile: self.quick.profile.clone(), ..Default::default() };
        let resolved = prepare_with_role(&catalog.scope, &catalog.cwd, &catalog.environment, &flags, self.tool.map(|t| t.as_str()), &selection, &[]);
        self.quick.error = None;
        self.quick.profile_label.clear();
        self.quick.source_label.clear();
        self.resolved_tool = None;
        match resolved {
            Ok((_, config)) => {
                self.resolved_tool = config.tool.parse().ok();
                if self.headless && self.resolved_tool != Some(crate::integrations::tool::Tool::Claude) {
                    self.headless_pty = true;
                }
                if let Some(p) = config.profile {
                    self.quick.profile_label = p.name;
                    self.quick.source_label = match p.selected_by.as_str() {
                        "role_profile" => "role",
                        "environment_profile" => "env",
                        "scope_default" => "default",
                        "tool_default" => "tool",
                        _ => "",
                    }
                    .into();
                }
            }
            Err(error) => self.quick.error = Some(format!("{error:#}")),
        }
    }

    pub(super) fn open_field(&mut self, field: LaunchField) {
        self.options_cursor = Some(field);
        self.quick.picker = None;
        if field == LaunchField::More {
            self.quick.advanced = !self.quick.advanced;
            if self.quick.advanced {
                self.options_cursor = Some(LaunchField::Tool);
            }
        } else {
            self.quick.advanced = false;
            self.quick.picker = Some(field);
            self.quick.query.clear();
            let selected = if field == LaunchField::Role { &self.quick.role } else { &self.quick.profile };
            self.quick.index = self.choices().iter().position(|(name, _)| *name == selected.as_deref()).unwrap_or(0);
        }
    }

    pub(super) fn choices(&self) -> Vec<(Option<&str>, &str)> {
        let role = self.quick.picker == Some(LaunchField::Role);
        let reset = if role { "No role" } else { "Auto" };
        let mut choices = vec![(None, reset)];
        if let Some(c) = &self.quick.catalog {
            let list = if role { &c.roles } else { &c.profiles };
            choices.extend(list.iter().filter(|(_, detail)| role || self.tool.is_none_or(|t| t.as_str() == detail)).map(|(name, detail)| (Some(name.as_str()), detail.as_str())));
        }
        let query = self.quick.query.to_lowercase();
        choices.retain(|(name, detail)| name.unwrap_or_default().to_lowercase().contains(&query) || detail.to_lowercase().contains(&query));
        choices
    }

    pub(super) fn choose(&mut self, index: usize) {
        let Some((name, _)) = self.choices().get(index).copied() else { return };
        let name = name.map(str::to_owned);
        match self.quick.picker {
            Some(LaunchField::Role) => self.quick.role = name,
            Some(LaunchField::Profile) => self.quick.profile = name,
            _ => return,
        }
        self.quick.picker = None;
        self.options_cursor = None;
        self.refresh_preview();
    }
}
