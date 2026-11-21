from pathlib import Path

from storage import AgentxError, read_document, read_yaml


def scope_chain(scope, root):
    scope, root = Path(scope).resolve(), Path(root).resolve()
    if not scope.is_dir() or not scope.is_relative_to(root):
        raise AgentxError(f"Scope must be an existing directory inside {root}: {scope}")
    current = scope
    while True:
        yield current
        if current == root:
            break
        current = current.parent


def strings(value, source):
    if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
        raise AgentxError(f"Expected a list of strings: {source}")
    return value


def resolve(reference, kind, scope, root, relative_to=None):
    reference = str(reference)
    explicit = "/" in reference or reference.startswith(".") or Path(reference).suffix in {".md", ".yaml", ".yml"}
    if explicit:
        path = Path(reference).expanduser()
        path = path if path.is_absolute() else Path(relative_to or Path.cwd()) / path
        if kind == "skills" and path.is_dir():
            path /= "SKILL.md"
        if not path.is_file():
            raise AgentxError(f"Missing {kind} source: {path}")
        return path.resolve()
    for directory in scope_chain(scope, root):
        for home in [directory / ".agent-guides/local", directory / ".agent-guides"]:
            candidates = ([home / kind / reference / "SKILL.md"] if kind == "skills" else
                          [home / kind / f"{reference}.md", home / kind / f"{reference}.yaml"])
            matches = [path for path in candidates if path.is_file()]
            if len(matches) > 1:
                raise AgentxError(f"Ambiguous {kind} reference {reference}: use an explicit path")
            if matches:
                return matches[0].resolve()
    raise AgentxError(f"No {kind} named {reference} between {scope} and {root}")


def load_role(reference, scope, root):
    if reference is None:
        return None, []
    path = resolve(reference, "roles", scope, root)
    if path.suffix in {".yaml", ".yml"}:
        metadata = read_yaml(path)
        prompt = metadata.get("content", "")
    else:
        document = read_document(path)
        metadata, prompt = document.metadata, document.body
    if metadata.get("schema") != "role" or not isinstance(metadata.get("role"), str):
        raise AgentxError(f"Role requires schema: role and a role name: {path}")
    if not isinstance(prompt, str):
        raise AgentxError(f"Role content must be text: {path}")
    skills = strings(metadata.get("skills", []), f"{path}:skills")
    return {"name": metadata["role"], "description": metadata.get("description", ""),
            "path": str(path), "instructions": prompt}, skills


def bind_skills(role, role_skills, extra_skills, scope, root):
    result, by_path, by_name = [], {}, {}
    declarations = [(ref, "role", Path(role["path"]).parent) for ref in role_skills]
    declarations += [(ref, "launch", Path.cwd()) for ref in extra_skills]
    for reference, origin, relative_to in declarations:
        path = resolve(reference, "skills", scope, root, relative_to)
        if str(path) in by_path:
            if origin not in by_path[str(path)]["via"]:
                by_path[str(path)]["via"].append(origin)
            continue
        metadata = read_document(path).metadata
        name, description = metadata.get("name"), metadata.get("description")
        if not isinstance(name, str) or not isinstance(description, str):
            raise AgentxError(f"Skill requires name and description: {path}")
        if name in by_name:
            raise AgentxError(f"Two bound skills share name {name}: {by_name[name]} and {path}")
        item = {"name": name, "description": description, "path": str(path), "via": [origin]}
        result.append(item)
        by_path[str(path)], by_name[name] = item, path
    return result


def merge_config(target, source):
    for key, value in source.items():
        if isinstance(value, dict) and isinstance(target.get(key), dict):
            merge_config(target[key], value)
        else:
            target[key] = value


def config_for(scope, root):
    config = {}
    for directory in reversed(list(scope_chain(scope, root))):
        path = directory / ".agentx/config.yaml"
        if path.exists():
            merge_config(config, read_yaml(path))
    return config


def discover(scope, root):
    resources = {"roles": {}, "skills": {}}
    for directory in scope_chain(scope, root):
        for home in [directory / ".agent-guides/local", directory / ".agent-guides"]:
            for kind in resources:
                source = home / kind
                paths = (sorted([*source.glob("*.md"), *source.glob("*.yaml")]) if kind == "roles"
                         else sorted(source.rglob("SKILL.md")))
                for path in paths:
                    reference = path.stem if kind == "roles" else path.parent.relative_to(source).as_posix()
                    if reference in resources[kind]:
                        continue
                    if kind == "roles":
                        selected = resolve(reference, kind, scope, root)
                        role, defaults = load_role(str(selected), scope, root)
                        entry = {"name": role["name"], "description": role["description"],
                                 "path": role["path"], "skills": defaults}
                    else:
                        metadata = read_document(path).metadata
                        if not isinstance(metadata.get("name"), str) or not isinstance(metadata.get("description"), str):
                            raise AgentxError(f"Skill requires name and description: {path}")
                        entry = {"name": metadata["name"], "description": metadata["description"],
                                 "path": str(path.resolve())}
                    resources[kind][reference] = entry
    return {"directory": str(Path(scope).resolve()),
            **{kind: list(items.values()) for kind, items in resources.items()}}
