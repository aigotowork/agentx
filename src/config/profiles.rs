//! Scoped reusable Provider argv and defaults; file edits share config's lock.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct LaunchProfile {
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct ProfileCatalog {
    pub profiles: BTreeMap<String, LaunchProfile>,
    pub default_profile: Option<String>,
    pub tool_defaults: BTreeMap<String, String>,
}

pub(crate) fn canonical_tool(value: &str) -> Result<String> {
    let tool = crate::runtime::launcher::LaunchTool::from_str(value)?;
    if !tool.spec().released {
        bail!("Unsupported tool '{value}'");
    }
    Ok(tool.tool().as_str().into())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || !name.as_bytes()[0].is_ascii_alphanumeric() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')) {
        bail!("Invalid profile name '{name}': use letters, digits, _ or -, starting with a letter or digit");
    }
    Ok(())
}

impl LaunchProfile {
    pub(crate) fn validate(&self) -> Result<()> {
        let tool = crate::runtime::launcher::LaunchTool::from_str(&self.tool)?;
        canonical_tool(&self.tool)?;
        let errors = crate::runtime::launcher::validate_tool_args(&tool, &self.args);
        if !errors.is_empty() {
            bail!("Invalid profile args: {}", errors.join("; "));
        }
        Ok(())
    }
}

impl ProfileCatalog {
    pub(crate) fn parse(content: &str) -> Result<Self> {
        let value: toml::Value = toml::from_str(content).context("Invalid config.toml")?;
        let Some(launch) = value.get("launch") else { return Ok(Self::default()) };
        let launch = launch.as_table().context("launch must be a table")?;
        let mut catalog = Self::default();
        if let Some(value) = launch.get("default_profile") {
            catalog.default_profile = Some(value.as_str().context("launch.default_profile must be a profile name")?.into());
        }
        if let Some(profiles) = launch.get("profiles") {
            for (name, value) in profiles.as_table().context("launch.profiles must be a table")? {
                validate_name(name)?;
                let mut profile: LaunchProfile = value.clone().try_into().with_context(|| format!("Invalid profile '{name}'"))?;
                profile.tool = canonical_tool(&profile.tool)?;
                catalog.profiles.insert(name.clone(), profile);
            }
        }
        for spec in crate::integrations::registry::ALL.iter().filter(|s| s.released) {
            if let Some(default) = launch.get(spec.name).and_then(|v| v.get("default_profile")) {
                catalog.tool_defaults.insert(spec.name.into(), default.as_str().context("tool default_profile must be a profile name")?.into());
            }
        }
        Ok(catalog)
    }

    pub(crate) fn load(scope: &Path) -> Result<Self> {
        let path = scope.join("config.toml");
        match std::fs::read_to_string(&path) {
            Ok(content) => Self::parse(&content).with_context(|| format!("Reading {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).with_context(|| format!("Reading {}", path.display())),
        }
    }

    pub(crate) fn get(&self, name: &str) -> Result<&LaunchProfile> {
        self.profiles.get(name).with_context(|| format!("Unknown profile '{name}'; use agentx profile list"))
    }

    fn validate_references(&self) -> Result<()> {
        if let Some(name) = &self.default_profile {
            self.get(name)?;
        }
        for (tool, name) in &self.tool_defaults {
            if self.get(name)?.tool != *tool {
                bail!("Default profile '{name}' does not use tool '{tool}'");
            }
        }
        Ok(())
    }
}

/// One locked file mutation; no ambient scope, managed storage or Provider I/O.
pub(crate) enum Edit {
    Create { name: String, profile: LaunchProfile },
    Update { name: String, description: Option<String>, args: Option<Vec<String>> },
    Delete { name: String },
    Default { tool: Option<String>, name: Option<String> },
}

pub(crate) fn edit(scope: &Path, operation: Edit) -> Result<()> {
    super::file::edit_config_at_path(&scope.join("config.toml"), |doc| {
        let result = (|| -> Result<()> {
            let mut catalog = ProfileCatalog::parse(&doc.to_string())?;
            match operation {
                Edit::Create { name, mut profile } => {
                    validate_name(&name)?;
                    if catalog.profiles.contains_key(&name) {
                        bail!("Profile '{name}' already exists");
                    }
                    profile.tool = canonical_tool(&profile.tool)?;
                    profile.validate()?;
                    catalog.profiles.insert(name.clone(), profile.clone());
                    write_profile(doc, &name, &profile);
                }
                Edit::Update { name, description, args } => {
                    let mut profile = catalog.get(&name)?.clone();
                    if description.is_some() {
                        profile.description = description.filter(|value| !value.is_empty());
                    }
                    if let Some(args) = args {
                        profile.args = args;
                    }
                    profile.validate()?;
                    catalog.profiles.insert(name.clone(), profile.clone());
                    write_profile(doc, &name, &profile);
                }
                Edit::Delete { name } => {
                    catalog.get(&name)?;
                    if catalog.default_profile.as_deref() == Some(&name) || catalog.tool_defaults.values().any(|value| value == &name) {
                        bail!("Profile '{name}' is a default; change it or use profile default --unset [--tool TOOL] first");
                    }
                    doc["launch"]["profiles"].as_table_like_mut().context("profiles must be a table")?.remove(&name);
                    catalog.profiles.remove(&name);
                }
                Edit::Default { tool, name } => {
                    if let Some(name) = &name {
                        catalog.get(name)?;
                    }
                    let item = name.as_ref().map(|value| toml_edit::value(value.as_str())).unwrap_or(toml_edit::Item::None);
                    if let Some(tool) = tool {
                        let tool = canonical_tool(&tool)?;
                        if let Some(name) = &name {
                            if catalog.get(name)?.tool != tool {
                                bail!("Profile '{name}' does not use tool '{tool}'");
                            }
                            catalog.tool_defaults.insert(tool.clone(), name.clone());
                        } else {
                            catalog.tool_defaults.remove(&tool);
                        }
                        doc["launch"][&tool]["default_profile"] = item;
                    } else {
                        catalog.default_profile = name;
                        doc["launch"]["default_profile"] = item;
                    }
                }
            }
            catalog.validate_references()
        })();
        result.map(|()| true).map_err(|error| format!("{error:#}"))
    })
    .map_err(anyhow::Error::msg)
}

fn write_profile(doc: &mut toml_edit::DocumentMut, name: &str, profile: &LaunchProfile) {
    let mut table = toml_edit::Table::new();
    table["tool"] = toml_edit::value(&profile.tool);
    if let Some(description) = &profile.description {
        table["description"] = toml_edit::value(description);
    }
    let mut args = toml_edit::Array::new();
    for arg in &profile.args {
        args.push(arg.as_str());
    }
    table["args"] = toml_edit::value(args);
    doc["launch"]["profiles"][name] = toml_edit::Item::Table(table);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_crud_defaults_and_atomic_refusal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "# retain me\ntimeout = 120\n").unwrap();
        let profile = LaunchProfile { tool: "codex".into(), description: None, args: vec!["--model".into(), "model with spaces".into(), "--future".into(), "".into()] };
        edit(dir.path(), Edit::Create { name: "daily".into(), profile: profile.clone() }).unwrap();
        assert_eq!(ProfileCatalog::load(dir.path()).unwrap().get("daily").unwrap(), &profile);
        edit(dir.path(), Edit::Default { name: Some("daily".into()), tool: None }).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(edit(dir.path(), Edit::Delete { name: "daily".into() }).is_err());
        assert!(edit(dir.path(), Edit::Default { name: Some("daily".into()), tool: Some("claude".into()) }).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        edit(dir.path(), Edit::Update { name: "daily".into(), description: Some("updated".into()), args: Some(vec![]) }).unwrap();
        assert!(ProfileCatalog::load(dir.path()).unwrap().get("daily").unwrap().args.is_empty());
        edit(dir.path(), Edit::Default { name: None, tool: None }).unwrap();
        edit(dir.path(), Edit::Delete { name: "daily".into() }).unwrap();
        assert!(ProfileCatalog::load(dir.path()).unwrap().profiles.is_empty());
        assert!(std::fs::read_to_string(&path).unwrap().contains("# retain me"));
    }

    #[test]
    fn malformed_or_unreadable_file_is_never_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "not valid [").unwrap();
        assert!(ProfileCatalog::load(dir.path()).is_err());
        assert!(edit(dir.path(), Edit::Default { name: None, tool: None }).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "not valid [");
    }
}
