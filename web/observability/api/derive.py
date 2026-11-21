"""Derive browser declarations from the finite native serde contract closure.

Run with Python 3 from any cwd. This is source generation, not runtime evidence.
Supported syntax is deliberately limited to the current native declarations;
unknown forms fail rather than emit an any/unknown schema. Native serde remains
the validator. Integer fields are bigint after the client lossless decode.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
SOURCES = [
    "src/observability/context.rs", "src/observability/runtime.rs",
    "src/observability/configuration.rs", "src/observability/events.rs",
    "src/daemon/observation.rs", "src/observability/web/api.rs",
    "src/agents/attachments.rs", "src/messaging/inbox.rs", "src/service/context.rs",
]


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
for source in SOURCES:
    text = clean((ROOT / source).read_text())
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

# The native UUID macro emits a custom string serde implementation; do not
# pretend its underlying Uuid tuple field describes that wire representation.
for name in re.findall(r"^uuid_identity!\((\w+)\);", (ROOT / SOURCES[1]).read_text(), re.M):
    declarations[name] = ("type", "", "=", "String", SOURCES[1])

pending = ["ReadCall", "ReadResult"]
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
    if re.fullmatch(r"[ui](8|16|32|64|size)", text): return "bigint"
    generic = re.fullmatch(r"(\w+)<(.*)>", text, re.S)
    if generic:
        name, values = generic.groups()
        values = [ty(part, params) for part in split(values)]
        if name == "Option": return "(" + values[0] + " | null)"
        if name == "Vec": return "Array<" + values[0] + ">"
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

output = "// Derived by derive.py from native serde declarations. Do not edit.\n// Integers are bigint in memory; client.ts maps explicit $integer markers.\n\n"
output += "\n\n".join(emitted[name] for name in sorted(emitted)) + "\n"
Path(__file__).with_name("native.ts").write_text(output)
