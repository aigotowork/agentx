//! Recursive public help, derived from the ordinary help sections and native clap trees.
//! This module never executes commands, user scripts, tools, or daemon acquisition.

use super::*;
use std::collections::HashMap;

/// Handle --help-all before any application bootstrap; None means ordinary execution.
pub(crate) fn dispatch(argv: &[String]) -> Option<i32> {
    let words = request(argv)?;
    Some(match render(&words) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(error) => {
            eprintln!("Error: {error}");
            2
        }
    })
}

pub(crate) fn is_requested(argv: &[String]) -> bool {
    request(argv).is_some()
}

fn request(argv: &[String]) -> Option<Vec<String>> {
    if !argv.iter().any(|word| word == "--help-all") {
        return None;
    }
    let values = value_options();
    let mut requested = false;
    let mut words = Vec::new();
    let mut i = 0;
    while i < argv.len() && argv[i] != "--" {
        let word = &argv[i];
        let takes_value = values.get(word).is_some_and(|optional| !optional || argv.get(i + 1).is_some_and(|value| !value.starts_with('-')));
        if crate::runtime::environment::takes_opaque_value(word) || takes_value {
            i += 2;
            continue;
        }
        if word == "--help-all" {
            requested = true;
        } else if !word.starts_with('-') {
            words.push(word.clone());
        }
        i += 1;
    }
    if words.first().is_some_and(|word| word == "help") {
        words.remove(0);
    }
    requested.then_some(words)
}

fn render(words: &[String]) -> Result<String, String> {
    let root = tree();
    let mut selected = &root;
    for word in words {
        let canonical = match (selected.path.as_str(), word.as_str()) {
            ("", name) => crate::cli::router::canonical_command(name),
            ("bundle", "preview") => "prepare",
            ("hooks", "install") => "add",
            ("hooks", "uninstall") => "remove",
            ("relay", "disconnect") => "off",
            _ => word,
        };
        if let Some(child) = selected.children.iter().find(|child| child.name() == canonical) {
            selected = child;
        } else if selected.accepts_operands {
            break;
        } else {
            return Err(format!("Unknown command path: agentx {}{}", if selected.path.is_empty() { String::new() } else { format!("{} ", selected.path) }, word));
        }
    }
    let mut output = format!("Command tree:\nagentx{}\n", if selected.path.is_empty() { String::new() } else { format!(" {}", selected.path) });
    selected.write_tree(&mut output, "");
    output.push_str("\nHelp controls (before --):\n  --help / -h                   Help for the current command\n  --help-all                    Full help for this command and all descendants\n\nShared invocation options (where supported):\n  --iam NAME                   Existing caller identity, not a new agent name\n  --go                          Skip confirmation/launch preview\n  --env-file PATH               Ordered dotenv files; repeatable\n  --env-override                Override inherited environment values\n  --no-env-override             Preserve inherited environment values (default)\n  AGENTX_DIR                    Data/config scope (default: ~/.agentx)\n  AGENTX_ENV_FILES              Comma-separated default dotenv list\n  AGENTX_ENV_OVERRIDE           Default environment override policy\n\nNative tool arguments belong after --. Native tool help and internal hooks are outside this public reference.\n");
    selected.write_pages(&mut output);
    Ok(output)
}

struct Node {
    path: String,
    help: String,
    children: Vec<Node>,
    accepts_operands: bool,
}

impl Node {
    fn name(&self) -> &str {
        self.path.rsplit(' ').next().unwrap_or("")
    }

    fn insert(&mut self, words: &[&str]) {
        let Some((name, remaining)) = words.split_first() else { return };
        let index = self.children.iter().position(|node| node.name() == *name).unwrap_or_else(|| {
            let path = format!("{} {name}", self.path);
            self.children.push(Node { help: manual_help(&path), accepts_operands: manual_accepts_operands(&path), path, children: Vec::new() });
            self.children.len() - 1
        });
        self.children[index].insert(remaining);
    }

    fn write_tree(&self, output: &mut String, indent: &str) {
        for (index, child) in self.children.iter().enumerate() {
            let last = index + 1 == self.children.len();
            output.push_str(&format!("{indent}{}{}\n", if last { "└─ " } else { "├─ " }, child.name()));
            child.write_tree(output, &format!("{indent}{}", if last { "   " } else { "│  " }));
        }
    }

    fn write_pages(&self, output: &mut String) {
        output.push_str(&format!("\n=== agentx{} ===\n", if self.path.is_empty() { String::new() } else { format!(" {}", self.path) }));
        for line in self.help.lines().filter(|line| !line.trim().is_empty()) {
            output.push_str(line.trim_end());
            output.push('\n');
        }
        for child in &self.children {
            child.write_pages(output);
        }
    }
}

fn tree() -> Node {
    let mut root = Node { path: String::new(), help: get_help_text(), children: Vec::new(), accepts_operands: false };
    for name in COMMAND_NAMES {
        if let Some(mut command) = native_command(name) {
            command.build();
            root.children.push(native_node(name.to_string(), &mut command));
            continue;
        }
        let mut node = Node { path: name.to_string(), help: get_command_help(name), children: Vec::new(), accepts_operands: manual_accepts_operands(name) };
        if let Some(sections) = manual_sections(name) {
            for (usage, _) in sections.iter().flat_map(|section| section.iter()) {
                let words: Vec<_> = usage.split_whitespace().take_while(|word| word.starts_with(|ch: char| ch.is_ascii_lowercase()) && word.chars().all(|ch| ch.is_ascii_lowercase() || matches!(ch, '-' | '_'))).collect();
                if words.first() == Some(name) && words.len() > 1 {
                    node.insert(&words[1..]);
                }
            }
        }
        root.children.push(node);
    }
    root
}

fn native_node(path: String, command: &mut clap::Command) -> Node {
    let accepts_operands = command.get_positionals().next().is_some();
    let help = compact_help(&command.render_long_help().to_string());
    let children = command.get_subcommands_mut().filter(|child| !child.is_hide_set() && child.get_name() != "help").map(|child| native_node(format!("{path} {}", child.get_name()), child)).collect();
    Node { path, help, children, accepts_operands }
}

fn manual_accepts_operands(path: &str) -> bool {
    if matches!(path, "resume" | "fork") {
        return true;
    }
    let root = path.split_whitespace().next().unwrap_or(path);
    manual_sections(root)
        .into_iter()
        .flatten()
        .flat_map(|section| section.iter())
        .any(|(usage, _)| usage.strip_prefix(&format!("{path} ")).and_then(|suffix| suffix.chars().next()).is_some_and(|first| first.is_ascii_uppercase() || matches!(first, '<' | '[' | '"')))
}

/// Select the narrowest published section, then retain only this path's usage
/// blocks. Options/descriptions remain attached to their declared usage, so a
/// leaf does not inherit sibling-only flags or fall back to its parent's page.
fn manual_help(path: &str) -> String {
    if let Some((_, sections)) = SUBCOMMAND_HELP.iter().find(|(topic, _)| *topic == path) {
        let root_prefix = format!("{} ", path.split_whitespace().next().unwrap_or(path));
        let selected = sections.iter().filter(|section| {
            let commands: Vec<_> = section.iter().filter(|(usage, _)| usage.starts_with(&root_prefix)).collect();
            commands.is_empty() || commands.iter().any(|(usage, _)| *usage == path || usage.starts_with(&format!("{path} ")))
        });
        let lines: Vec<_> = selected.flat_map(|section| format_entries(section)).collect();
        return compact_help(&expand_hook_tools(&lines.join("\n")));
    }
    let owner = SUBCOMMAND_HELP.iter().filter(|(topic, _)| path.starts_with(&format!("{topic} "))).max_by_key(|(topic, _)| topic.len()).map(|(topic, _)| *topic).unwrap_or_else(|| path.split_whitespace().next().unwrap_or(path));
    let mut active = false;
    let mut lines = vec!["Usage:".to_string()];
    for (usage, description) in manual_sections(owner).into_iter().flatten().flat_map(|section| section.iter()) {
        if usage.starts_with(owner) {
            active = *usage == path || usage.starts_with(&format!("{path} "));
        }
        if active {
            lines.push(format_entry(usage, description));
        }
    }
    let text = compact_help(&expand_hook_tools(&lines.join("\n")));
    text.replace("{transcript_agents}", &crate::transcript::transcript_tool_names().join(" | "))
}

/// Option arity comes from the same native declarations and help entries as
/// the displayed reference. Required text values may themselves be --help-all.
fn value_options() -> HashMap<String, bool> {
    let mut values = HashMap::new();
    for name in COMMAND_NAMES {
        if let Some(mut command) = native_command(name) {
            command.build();
            native_value_options(&command, &mut values);
        }
        if let Some(sections) = manual_sections(name) {
            for (usage, _) in sections.iter().flat_map(|section| section.iter()) {
                let words: Vec<_> = usage.split_whitespace().collect();
                for pair in words.windows(2) {
                    if pair[0].starts_with('-') && pair[1].starts_with(|ch: char| ch.is_ascii_uppercase() || matches!(ch, '<' | '"' | '[')) {
                        values.insert(pair[0].to_string(), pair[1].starts_with('['));
                    }
                }
            }
        }
    }
    values
}

fn native_value_options(command: &clap::Command, values: &mut HashMap<String, bool>) {
    for arg in command.get_arguments().filter(|arg| arg.get_action().takes_values()) {
        let optional = arg.get_num_args().is_some_and(|range| range.min_values() == 0);
        if let Some(long) = arg.get_long() {
            values.insert(format!("--{long}"), optional);
        }
        if let Some(short) = arg.get_short() {
            values.insert(format!("-{short}"), optional);
        }
    }
    for child in command.get_subcommands() {
        native_value_options(child, values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(input: &[&str]) -> Vec<String> {
        input.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn help_all_root_covers_public_commands_and_native_descendants() {
        let text = render(&[]).unwrap();
        for name in COMMAND_NAMES {
            assert!(text.contains(&format!("=== agentx {name} ===")), "missing {name}");
        }
        for detail in ["=== agentx work report ===", "--assignment", "--brief", "=== agentx daemon restart ===", "=== agentx observe compare ===", "--left-file", "--reply-to", "--count"] {
            assert!(text.contains(detail), "missing {detail}");
        }
        assert!(!text.contains("=== agentx daemon serve ==="));
        assert!(!text.contains("=== agentx pty ==="));
    }

    #[test]
    fn help_all_subtrees_include_deep_children_without_unrelated_commands() {
        let events = render(&words(&["events", "sub"])).unwrap();
        assert!(events.contains("=== agentx events sub list ==="));
        assert!(events.contains("--once"));
        assert!(!events.contains("=== agentx events unsub ==="));
        assert!(!events.contains("agentx events unsub <id>"));
        assert!(!events.contains("=== agentx launch ==="));
        let list = render(&words(&["events", "sub", "list"])).unwrap();
        assert!(list.contains("--device"));
        assert!(!list.contains("--once"));
        let work = render(&words(&["work", "report"])).unwrap();
        assert!(work.contains("--assignment"));
        assert!(!work.contains("=== agentx work apply ==="));
        let bootstrap = render(&words(&["config", "bootstrap"])).unwrap();
        assert!(bootstrap.contains("=== agentx config bootstrap show ==="));
        assert!(bootstrap.contains("=== agentx config bootstrap render ==="));
        assert!(!bootstrap.contains("=== agentx config environment ==="));
    }

    #[test]
    fn help_all_preserves_native_tails_and_text_values() {
        for input in [
            vec!["launch", "--tool", "codex", "--", "--help-all"],
            vec!["send", "--", "--help-all"],
            vec!["launch", "--tool", "codex", "--agentx-prompt", "--help-all"],
            vec!["events", "--sql", "--help-all"],
            vec!["observe", "compare", "--field", "--help-all"],
        ] {
            assert_eq!(request(&words(&input)), None, "{input:?}");
        }
        for (input, expected) in [
            (vec!["--help-all"], vec![]),
            (vec!["events", "--last", "5", "--help-all"], vec!["events"]),
            (vec!["events", "--wait", "--help-all"], vec!["events"]),
            (vec!["launch", "--tool", "codex", "--count", "2", "--help-all"], vec!["launch"]),
            (vec!["help", "work", "report", "--help-all"], vec!["work", "report"]),
        ] {
            assert_eq!(request(&words(&input)), Some(words(&expected)), "{input:?}");
        }
    }

    #[test]
    fn help_all_unknown_paths_fail_and_existing_aliases_select_canonical_help() {
        assert!(render(&words(&["codex"])).is_err());
        assert!(render(&words(&["events", "missing"])).is_err());
        assert_eq!(render(&words(&["r"])), render(&words(&["resume"])));
        assert_eq!(render(&words(&["bundle", "preview"])), render(&words(&["bundle", "prepare"])));
        assert_eq!(render(&words(&["resume", "luna"])), render(&words(&["resume"])));
        assert_eq!(render(&words(&["transcript", "luna"])), render(&words(&["transcript"])));
        // Other tests can change AGENTX_DIR between renders; compare the help
        // content while leaving the environment-dependent file paths out.
        let config_help = |path: &[&str]| render(&words(path)).unwrap().lines().filter(|line| !line.trim_start().starts_with("Files:")).map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(config_help(&["config", "terminal"]), config_help(&["config"]));
    }
}
