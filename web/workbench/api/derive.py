"""Derive browser declarations from the finite native serde contract closure.

Run with Python 3 from any cwd. This is source generation, not runtime evidence.
Supported syntax is deliberately limited to the current native declarations;
unknown forms fail rather than emit an any/unknown schema. Native serde remains
the validator. Integer fields are bigint after the client lossless decode.
"""
from pathlib import Path
import argparse
import re

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--sources", nargs="+", default=["src/collaboration/service.rs", "src/collaboration/web/api.rs", "src/agents/attachments.rs"])
parser.add_argument("--roots", nargs="+", default=["ReadCall", "ApplyCall", "Descriptor", "Response"])
parser.add_argument("--output", type=Path, default=Path(__file__).with_name("native.ts"))
parser.add_argument("--numbers", choices=["bigint", "number"], default="bigint")
options = parser.parse_args()
SOURCES = options.sources


def split(text, delimiter=","):
    parts, start, stack = [], 0, []
    for i, ch in enumerate(text):
        if ch in "({[<": stack.append(ch)
        elif ch in ")}]>": stack.pop()
        elif ch == delimiter and not stack:
            parts.append(text[start:i].strip())
            start = i + 1
    return [p for p in parts + [text[start:].strip()] if p]


def balanced(text, start):
    opening = text[start]
    closing = {"{": "}", "(": ")", "[": "]", "<": ">"}[opening]
    depth = 0
    for i in range(start, len(text)):
        if text[i] == opening: depth += 1
        elif text[i] == closing:
            depth -= 1
            if not depth: return text[start + 1:i], i + 1
    raise ValueError("unbalanced native declaration")


def clean(text):
    text = re.sub(r"//[^\n]*", "", text)
    while "#[" in text:
        start = text.index("#[")
        _, end = balanced(text, start + 1)
        text = text[:start] + text[end:]
    return text


declarations = {}
enum_renames = {}
for source in SOURCES:
    raw = (ROOT / source).read_text()
    for rename in re.finditer(r'#\[serde\(rename_all\s*=\s*"([a-z_]+)"\)\]\s*(?:pub(?:\(crate\))?\s+)?enum\s+(\w+)', raw):
        enum_renames[rename.group(2)] = rename.group(1)
    text = clean(raw)
    pattern = r"(?:pub(?:\(crate\))?\s+)?(struct|enum|type)\s+(\w+)\s*(<[^>]+>)?\s*"
    for match in re.finditer(pattern, text):
        kind, name, generics = match.groups()
        start = match.end()
        if text[start:start + 1] in ("{", "("):
            body, _ = balanced(text, start)
            declarations[name] = (kind, generics or "", text[start], body, source)
        elif kind == "type" and text[start:start + 1] == "=":
            end = text.index(";", start)
            declarations[name] = (kind, generics or "", "=", text[start + 1:end].strip(), source)

pending = options.roots[:]
emitted = {}
primitive = {"String": "string", "str": "string", "PathBuf": "string", "bool": "boolean", "Uuid": "string"}


def ty(text, params=()):
    text = re.sub(r"\bpub(?:\(crate\))?\s+", "", text).strip()
    if text in params: return text
    if text == "()": return "null"
    if text.startswith("["):
        item, count = text[1:-1].split(";")
        return "[" + ", ".join([ty(item.strip(), params)] * int(count.strip())) + "]"
    if text.startswith("("):
        return "[" + ", ".join(ty(part, params) for part in split(text[1:-1])) + "]"
    text = re.sub(r"(?:\w+::)+", "", text)
    if text in primitive: return primitive[text]
    if text in ("f32", "f64"): return "number"
    if re.fullmatch(r"[ui](8|16|32|64|size)", text): return options.numbers
    generic = re.fullmatch(r"(\w+)<(.*)>", text, re.S)
    if generic:
        name, values = generic.groups()
        values = [ty(part, params) for part in split(values)]
        if name == "Box":
            if len(values) != 1: raise ValueError("Box requires one native type")
            return values[0]
        if name == "Option": return "(" + values[0] + " | null)"
        if name == "Vec": return "Array<" + values[0] + ">"
        if name in ("BTreeMap", "HashMap"):
            if len(values) != 2 or values[0] != "string": raise ValueError("JSON map requires string keys")
            return "Record<string, " + values[1] + ">"
        if name == "Result": return "({ Ok: " + values[0] + " } | { Err: " + values[1] + " })"
        pending.append(name)
        return name + "<" + ", ".join(values) + ">"
    if text not in declarations: raise ValueError("unsupported native type: " + text)
    pending.append(text)
    return text


def fields(body, params):
    result = []
    for field in split(body):
        name, value = field.split(":", 1)
        name = re.sub(r"^pub(?:\(crate\))?\s+", "", name.strip())
        if not re.fullmatch(r"\w+", name): raise ValueError("unsupported field " + name)
        result.append(name + ": " + ty(value, params))
    return "{ " + "; ".join(result) + " }"


while pending:
    name = pending.pop(0)
    if name in emitted: continue
    kind, generic, shape, body, source = declarations[name]
    params = tuple(split(generic[1:-1])) if generic else ()
    if kind == "type": result = ty(body, params)
    elif kind == "struct":
        if shape == "{": result = fields(body, params)
        else:
            parts = split(body)
            result = ty(parts[0], params) if len(parts) == 1 else "[" + ", ".join(ty(part, params) for part in parts) + "]"
    else:
        variants = []
        for variant in split(body):
            m = re.match(r"(\w+)\s*(.*)", variant, re.S)
            tag, rest = m.groups()
            rename = enum_renames.get(name)
            if rename == "snake_case":
                tag = re.sub(r"(?<!^)(?=[A-Z])", "_", tag).lower()
            elif rename:
                raise ValueError("unsupported serde enum rename_all " + rename)
            if not rest: variants.append('"' + tag + '"')
            elif rest.startswith("{"):
                inner, _ = balanced(rest, 0)
                variants.append("{ " + tag + ": " + fields(inner, params) + " }")
            elif rest.startswith("("):
                inner, _ = balanced(rest, 0)
                values = split(inner)
                value = ty(values[0], params) if len(values) == 1 else "[" + ", ".join(ty(value, params) for value in values) + "]"
                variants.append("{ " + tag + ": " + value + " }")
            else: raise ValueError("unsupported enum variant " + variant)
        result = " | ".join(variants)
    emitted[name] = f"// Source: {source}\nexport type {name}{generic} = {result};"

integer_note = "Integers are bigint in memory; client.ts maps explicit $integer markers." if options.numbers == "bigint" else "Bounded numeric fields use numbers; identities and cursors use native string fields."
output = "// Derived by derive.py from native serde declarations. Do not edit.\n// " + integer_note + "\n\n"
output += "\n\n".join(emitted[name] for name in sorted(emitted)) + "\n"
options.output.parent.mkdir(parents=True, exist_ok=True)
options.output.write_text(output)
