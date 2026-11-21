//! Literal prompt parsing. No launch-profile or participant state is read here.
use super::{Namespace, RoleDefinition};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Override {
    profile: Option<String>,
    prompt: Option<String>,
    prompt_file: Option<PathBuf>,
    description: Option<String>,
    #[serde(default)]
    default: bool,
    #[serde(default)]
    disabled: bool,
}

#[derive(Default, Deserialize)]
struct Metadata {
    schema: Option<String>,
    role: Option<String>,
    description: Option<String>,
}

fn metadata(content: &str) -> Result<Metadata> {
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return Ok(Metadata::default());
    }
    let mut yaml = Vec::new();
    for line in lines {
        if line == "---" {
            return serde_yaml_ng::from_str(&yaml.join("\n")).context("Parsing role Markdown metadata");
        }
        yaml.push(line);
    }
    bail!("Unterminated role Markdown metadata");
}

pub(super) fn is_role(path: &Path) -> Result<bool> {
    Ok(metadata(&std::fs::read_to_string(path)?)?.schema.as_deref() == Some("role"))
}

fn file_prompt(path: &Path) -> Result<String> {
    let path = std::fs::canonicalize(path).with_context(|| format!("Resolving role prompt {}", path.display()))?;
    let content = std::fs::read_to_string(&path).with_context(|| format!("Reading UTF-8 role prompt {}", path.display()))?;
    Ok(format!("[Role prompt file: {}]\nResolve relative paths in this role prompt from this file's parent directory.\n\n{content}", path.display()))
}

fn markdown(path: &Path, name: &str) -> Result<(String, Option<String>)> {
    let content = std::fs::read_to_string(path).context("Reading UTF-8 role Markdown")?;
    let metadata = metadata(&content)?;
    if metadata.schema.as_deref() != Some("role") {
        bail!("Markdown prompt inheritance requires schema: role");
    }
    if metadata.role.as_ref().is_some_and(|role| role != name) {
        bail!("Markdown role identity must match filename '{name}'");
    }
    Ok((file_prompt(path)?, metadata.description))
}

/// None is an explicit tombstone: the caller must suppress farther definitions.
pub(super) fn read(namespace: &Namespace, name: &str, source: &Path) -> Result<Option<RoleDefinition>> {
    let read = || -> Result<_> {
        let path = std::fs::canonicalize(source).with_context(|| format!("Resolving role {}", source.display()))?;
        let (profile, prompt, description, default) = if source.extension().is_some_and(|value| value == "md") {
            let (prompt, description) = markdown(&path, name)?;
            (None, prompt, description, false)
        } else {
            let definition: Override = toml::from_str(&std::fs::read_to_string(&path)?).context("Parsing role TOML")?;
            if definition.disabled {
                if definition.profile.is_some() || definition.prompt.is_some() || definition.prompt_file.is_some() || definition.description.is_some() || definition.default {
                    bail!("disabled = true must be the only role setting");
                }
                return Ok(None);
            }
            if name.contains('.') && definition.default {
                bail!("Auxiliary roles cannot declare default = true; use a public role as the directory endpoint");
            }
            let (prompt, inherited_description) = match (definition.prompt, definition.prompt_file) {
                (Some(prompt), None) => (prompt, None),
                (None, Some(file)) => (file_prompt(&path.parent().expect("canonical role parent").join(file))?, None),
                (None, None) => {
                    let base = namespace.markdown.as_ref().context("Exactly one of prompt or prompt_file is required without a same-workspace Markdown role")?.join(format!("{name}.md"));
                    markdown(&base, name)?
                }
                _ => bail!("Only one of prompt or prompt_file is allowed"),
            };
            (definition.profile, prompt, definition.description.or(inherited_description), definition.default)
        };
        Ok(Some(RoleDefinition { name: name.into(), path, namespace: namespace.identity(), profile, prompt, description, default }))
    };
    read().with_context(|| format!("Invalid role {}", source.display()))
}
