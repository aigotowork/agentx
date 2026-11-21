#!/usr/bin/env python3
"""Compile local image briefs into reviewable jobs; see ../references/tool.md."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sys


DEFAULT_CATALOG = Path(__file__).resolve().parent.parent / "catalog"
DEFAULT_FORMATS = ["system-board", "screen", "component-states", "spot-illustration"]
DEFAULT_SUBJECTS = ["开始", "空状态", "完成"]
AXES = ("density", "geometry", "typography", "color", "material", "motion")
STATIC_AXES = AXES[:-1]
ID_PATTERN = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\Z")


class InputError(ValueError):
    """An actionable brief, catalogue or output-path error."""


def require(condition, location, message):
    if not condition:
        raise InputError(f"{location}: {message}")


def text_value(value, location):
    require(isinstance(value, str) and bool(value.strip()), location,
            "expected a non-blank string")
    try:
        value.encode("utf-8")
    except UnicodeEncodeError:
        raise InputError(f"{location}: expected valid UTF-8 text") from None
    return value


def strings(value, location, nonempty=False, unique=False):
    require(isinstance(value, list), location, "expected an array of strings")
    require(not nonempty or bool(value), location, "expected at least one item")
    for index, item in enumerate(value):
        text_value(item, f"{location}[{index}]")
    require(not unique or len(set(value)) == len(value), location, "duplicate values")
    return value


def identifier(value, location):
    text_value(value, location)
    require(ID_PATTERN.fullmatch(value), location, "expected lowercase ASCII kebab-case ID")
    return value


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "JSON", f"duplicate object key {key!r}")
        result[key] = value
    return result


def reject_constant(value):
    raise InputError(f"JSON: non-finite number {value} is not supported")


def read_json(path):
    path = Path(path).resolve()
    raw = path.read_bytes()
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object,
                           parse_constant=reject_constant)
        json_bytes(value)  # Reject escaped lone surrogates and overflowing JSON numbers too.
    except (UnicodeError, ValueError) as error:
        raise InputError(f"{path}: invalid UTF-8 JSON: {error}") from None
    require(isinstance(value, dict), path, "expected a JSON object")
    return value, raw


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2,
                       allow_nan=False) + "\n").encode("utf-8")


def source_record(path, raw, snapshot):
    return {"path": str(Path(path).resolve()), "sha256": digest(raw),
            "bytes": len(raw), "snapshot": snapshot}


def catalogue_items(document, location, kind):
    require(type(document.get("version")) is int and document["version"] == 1,
            location, "version must be integer 1")
    items = document.get("items")
    require(isinstance(items, list) and bool(items), location, "items must be a nonempty array")
    result = {}
    for index, item in enumerate(items):
        label = f"{location}.items[{index}]"
        require(isinstance(item, dict), label, "expected an object")
        item_id = identifier(item.get("id"), f"{label}.id")
        require(item_id not in result, label, f"duplicate ID {item_id!r}")
        for field in ("name", "prompt"):
            text_value(item.get(field), f"{label}.{field}")
        if kind == "formats":
            require(item.get("kind") in ("system", "screen", "states", "asset"),
                    f"{label}.kind", "expected system, screen, states or asset")
            for field in ("width", "height"):
                require(type(item.get(field)) is int and item[field] > 0,
                        f"{label}.{field}", "expected a positive integer in pixels")
            strings(item.get("checks"), f"{label}.checks")
        else:
            for field in ("summary", "evidence"):
                text_value(item.get(field), f"{label}.{field}")
            for field in ("aliases", "suitable_for", "avoid_for"):
                strings(item.get(field), f"{label}.{field}")
            if kind == "styles":
                axes = item.get("axes")
                require(isinstance(axes, dict), f"{label}.axes", "expected an object")
                for axis in AXES:
                    text_value(axes.get(axis), f"{label}.axes.{axis}")
                strings(item.get("asset_styles"), f"{label}.asset_styles", True, True)
            else:
                strings(item.get("constraints"), f"{label}.constraints")
                require(item.get("background") in ("opaque", "transparent"),
                        f"{label}.background", "expected opaque or transparent")
        result[item_id] = item
    return result


def check_refs(values, available, location):
    for value in values:
        require(value in available, location,
                f"unknown ID {value!r}; available: {', '.join(available)}")


def load_catalogues(directory):
    catalogues, sources, snapshots = {}, {}, {}
    for kind in ("styles", "asset-styles", "formats"):
        path = directory / f"{kind}.json"
        document, raw = read_json(path)
        catalogues[kind] = catalogue_items(document, str(path), kind)
        snapshot = f"sources/catalog/{kind}.json"
        sources[kind] = source_record(path, raw, snapshot)
        snapshots[snapshot] = raw
    styles, media, formats = (catalogues[key] for key in ("styles", "asset-styles", "formats"))
    require(len(styles) >= 8, directory, "styles catalogue requires at least 8 styles")
    check_refs(["calm-editorial", "precise-tech", "warm-playful"], styles, "required styles")
    check_refs(["monoline", "ink-wash"], media, "required asset styles")
    check_refs(DEFAULT_FORMATS, formats, "required formats")
    for format_id, kind in zip(DEFAULT_FORMATS, ("system", "screen", "states", "asset")):
        require(formats[format_id]["kind"] == kind, format_id, f"kind must be {kind}")
    if "asset-family-board" in formats:
        require(formats["asset-family-board"]["kind"] == "system", "asset-family-board", "kind must be system")
    for style in styles.values():
        check_refs(style["asset_styles"], media, f"styles.{style['id']}.asset_styles")
    return catalogues, sources, snapshots


def validate_brief(brief, catalogues):
    for field in ("id", "product", "audience", "intent", "scenario"):
        text_value(brief.get(field), f"brief.{field}")
    strings(brief.get("styles"), "brief.styles", True, True)
    check_refs(brief["styles"], catalogues["styles"], "brief.styles")
    resolved = dict(brief)
    for field, default, nonempty, unique in (
            ("constraints", [], False, False),
            ("asset_subjects", DEFAULT_SUBJECTS, True, False),
            ("formats", DEFAULT_FORMATS, True, True)):
        resolved[field] = strings(brief.get(field, list(default)), f"brief.{field}", nonempty, unique)
    check_refs(resolved["formats"], catalogues["formats"], "brief.formats")
    if "asset-family-board" in resolved["formats"]:
        require(len(resolved["asset_subjects"]) <= 3, "brief.asset_subjects",
                "asset-family-board supports at most 3 subjects; choose up to 3 in a separate brief, "
                "or remove asset-family-board from formats")
    if "asset_style" in brief:
        identifier(brief["asset_style"], "brief.asset_style")
        check_refs([brief["asset_style"]], catalogues["asset-styles"], "brief.asset_style")
    validate_render(resolved, catalogues)
    return resolved


def is_illustration(form):
    return form["kind"] == "asset" or form["id"] == "asset-family-board"


def validate_render(brief, catalogues):
    render = brief.get("render")
    require(isinstance(render, dict), "brief.render",
            "explicit render object required; translate visible content using contract v2 (no legacy fallback)")
    require(type(render.get("version")) is int and render["version"] == 1,
            "brief.render.version", "expected integer 1")
    formats = render.get("formats")
    require(isinstance(formats, dict), "brief.render.formats", "expected an object keyed by format ID")
    check_refs(formats, catalogues["formats"], "brief.render.formats")
    for format_id in brief["formats"]:
        require(format_id in formats, "brief.render.formats", f"missing explicit render for {format_id!r}")
    for format_id, instructions in formats.items():
        label = f"brief.render.formats.{format_id}"
        require(isinstance(instructions, dict), label, "expected an object")
        for field in ("constraints", "unknowns"):
            strings(instructions.get(field), f"{label}.{field}")
        if is_illustration(catalogues["formats"][format_id]):
            require("content" not in instructions, label,
                    "illustration content must come from render.asset_subjects, not content")
        else:
            strings(instructions.get("content"), f"{label}.content", True)
    uses_assets = any(is_illustration(catalogues["formats"][key]) for key in brief["formats"])
    if uses_assets or "asset_subjects" in render:
        subjects = strings(render.get("asset_subjects"), "brief.render.asset_subjects", True)
        require(len(subjects) == len(brief["asset_subjects"]), "brief.render.asset_subjects",
                "must align by index and count with top-level asset_subjects")
    if uses_assets or "asset_style_constraints" in render:
        styles = render.get("asset_style_constraints")
        require(isinstance(styles, dict), "brief.render.asset_style_constraints", "expected an object keyed by style ID")
        check_refs(styles, catalogues["styles"], "brief.render.asset_style_constraints")
        if uses_assets:
            for style_id in brief["styles"]:
                require(style_id in styles, "brief.render.asset_style_constraints",
                        f"missing explicit visual constraints for {style_id!r}")
        for style_id, constraints in styles.items():
            strings(constraints, f"brief.render.asset_style_constraints.{style_id}", True)


def job_id(brief_id, style_id, format_id, subject_index=None, subject=None):
    identity = [brief_id, style_id, format_id, subject_index, subject]
    return "job-" + digest(json_bytes(identity))[:24]


def references(kind, board_id):
    if kind in ("screen", "states") and board_id:
        return [board_id], [{"role": "system-design-anchor", "status": "unbound",
                             "source_job_id": board_id, "required_before_execution": True,
                             "instruction": "系统板生成并经人工检查后，执行器绑定真实图与回执。"}]
    if kind == "asset":
        role = {"role": "style-anchor", "status": "unbound", "required_before_execution": False,
                "instruction": "可独立按文字配方制作；若后来采用真实风格锚图，另行记录绑定与来源。"}
        if board_id:
            role["source_job_id"] = board_id
        return [], [role]
    return [], []


def render_prompt(brief, style, form, media, job):
    """Compile only declared drawing fields; orchestration remains in review/manifest."""
    family_board = form["id"] == "asset-family-board"
    instructions = brief["render"]["formats"][form["id"]]
    lines = ["# 单项绘制指令", "", "只画一个完整一致的视觉家族；不要画第二套/第三套方案、A/B/C 方向或不同风格比较列。",
             "以下是绘制指令，不作为画面文字；只呈现明确列出的内容，不补造未给定的业务信息。", "",
             f"目标画布：{form['width']} × {form['height']} px。", "", "## 格式构图", form["prompt"], ""]
    if is_illustration(form):
        if family_board:
            lines.append("## 同一家族的全部描绘主题（保持原顺序）")
            lines.extend(f"{index + 1}. {subject}" for index, subject in enumerate(job["render_content"]))
        else:
            lines.extend(["## 本项唯一描绘内容", job["render_subject"]])
        lines.extend(["", "## 素材家族视觉约束"])
        lines.extend(f"- {constraint}" for constraint in brief["render"]["asset_style_constraints"][style["id"]])
        lines.extend(["", "## 媒介画法", media["prompt"]])
        lines.extend(f"- {constraint}" for constraint in media["constraints"])
        if family_board:
            lines.append("本图集背景：opaque，统一中性实底；上述媒介的单体/透明要求适用于每格素材概念，整张按图集格式构图，不画棋盘格。")
        else:
            lines.append(f"背景要求：{media['background']}。")
        lines.append("不绘制 UI、导航、文案或状态网格；不加入其他未列出的素材主题。")
    else:
        lines.append("## 本图明确可见的内容")
        lines.extend(f"- {item}" for item in job["render_content"])
        lines.extend(["", "## 本图唯一风格", style["prompt"]])
        lines.extend(f"{axis}: {style['axes'][axis]}" for axis in STATIC_AXES)
    lines.extend(["", "## 本图视觉约束"])
    lines.extend(f"- {constraint}" for constraint in instructions["constraints"])
    lines.append("不添加内部任务、作者、版本、模型、来源或验收说明。")
    return "\n".join(lines) + "\n"


def plan_batch(brief, catalogues, sources, snapshots):
    resolved = validate_brief(brief, catalogues)
    jobs, payloads = [], dict(snapshots)
    for style_id in resolved["styles"]:
        style = catalogues["styles"][style_id]
        media_id = resolved.get("asset_style", style["asset_styles"][0])
        media = catalogues["asset-styles"][media_id]
        board_id = (job_id(brief["id"], style_id, "system-board")
                    if "system-board" in resolved["formats"] else None)
        for format_id in resolved["formats"]:
            form = catalogues["formats"][format_id]
            subjects = list(enumerate(resolved["asset_subjects"])) if form["kind"] == "asset" else [(None, None)]
            for index, subject in subjects:
                identity = job_id(brief["id"], style_id, format_id, index, subject)
                dependencies, roles = references(form["kind"], board_id)
                job = {"id": identity, "style_id": style_id, "format_id": format_id, "kind": form["kind"],
                       "prompt_file": f"prompts/{identity}.txt", "width": form["width"], "height": form["height"],
                       "status": "prepared", "depends_on": dependencies, "reference_roles": roles,
                       "reference_policy": "await-system-board" if dependencies else "text-recipe",
                       "checks": form["checks"]}
                if subject is not None:
                    job.update(subject=subject, subject_index=index, asset_style_id=media_id,
                               background=media["background"], render_subject=resolved["render"]["asset_subjects"][index])
                    job["render_content"] = [job["render_subject"]]
                elif format_id == "asset-family-board":
                    job.update(subjects=resolved["asset_subjects"], asset_style_id=media_id,
                               background="opaque", medium_background_requirement=media["background"],
                               render_content=resolved["render"]["asset_subjects"])
                else:
                    job["render_content"] = resolved["render"]["formats"][format_id]["content"]
                raw_prompt = render_prompt(resolved, style, form, media, job).encode("utf-8")
                job["prompt_sha256"] = digest(raw_prompt)
                require(job["prompt_file"] not in payloads, identity, "job identity collision")
                payloads[job["prompt_file"]] = raw_prompt
                jobs.append(job)
    known = {job["id"] for job in jobs}
    for job in jobs:
        check_refs(job["depends_on"], known, job["id"])
    source_hashes = {key: value["sha256"] for key, value in sources.items()}
    selected = {"styles": [catalogues["styles"][key] for key in resolved["styles"]],
                "formats": [catalogues["formats"][key] for key in resolved["formats"]],
                "asset-styles": [catalogues["asset-styles"][key] for key in dict.fromkeys(
                    resolved.get("asset_style", catalogues["styles"][style_id]["asset_styles"][0])
                    for style_id in resolved["styles"])]}
    manifest = {"version": 2, "status": "prepared", "batch_id": "batch-" + digest(json_bytes(source_hashes))[:24],
                "brief": brief, "resolved_brief": resolved, "sources": sources,
                "selected_catalogue": selected,
                "selection": {"basis": "explicit-brief-selection", "style_ids": resolved["styles"],
                              "note": "风格为 brief 显式选择；工具没有推断审美适合度或用户批准。",
                              "asset_style_basis": "explicit-brief-selection" if "asset_style" in brief else "first-style-recipe"},
                "jobs": jobs}
    payloads["review.md"] = render_review(manifest, catalogues).encode("utf-8")
    payloads["manifest.json"] = json_bytes(manifest)
    return manifest, payloads


def inline(value):
    return str(value).replace("\r", " ").replace("\n", " ").replace("|", "\\|")


def render_review(manifest, catalogues):
    brief = manifest["resolved_brief"]
    lines = ["# 图片批次文字审阅", "", f"产品：{inline(brief['product'])}", "",
             f"批次 `{manifest['batch_id']}` 共 {len(manifest['jobs'])} 项；全部为 **prepared / 尚未生成**。",
             "这是目录与文字配方审阅，不是模型图像预览。风格由 brief 显式选择，准备成功不等于审美认可。", "",
             "## 怎样比较", "", "固定同一产品、用户、目标、场景、文案和数据，逐项比较以下视觉结构；不要仅比较配色。", "",
             "| 风格 | 密度 | 几何 | 字体 | 颜色 | 材质 | 动效 | 证据 |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for style_id in brief["styles"]:
        style = catalogues["styles"][style_id]
        cells = [f"{style['name']} ({style_id})"] + [style["axes"][axis] for axis in AXES] + [style["evidence"]]
        lines.append("| " + " | ".join(inline(cell) for cell in cells) + " |")
    lines.extend(["", "## 执行与参考图", "",
                  "先处理无依赖任务；screen/states 若依赖 system-board，须在系统板生成、检查并绑定真实图后执行。",
                  "素材依赖列表为空，可独立制作；style-anchor 仅声明未来可选参考。所有 reference_roles 均为 unbound，没有伪造参考图路径。"])
    if "system-board" not in brief["formats"]:
        lines.append("本批未选系统板，其他格式仅受文字配方约束；视觉一致性尚未验证。")
    lines.extend(["", "| 提示词 | 风格 / 格式 | 素材主题 | 目标画布 | 依赖 |", "| --- | --- | --- | --- | --- |"])
    for job in manifest["jobs"]:
        cells = [f"[{job['id']}]({job['prompt_file']})", f"{job['style_id']} / {job['format_id']}",
                 inline(job.get("subject", "；".join(job.get("subjects", [])) or "—")), f"{job['width']} × {job['height']} px",
                 ", ".join(job["depends_on"]) or "无（文字配方）"]
        lines.append("| " + " | ".join(cells) + " |")
    lines.extend(["", "## 来源与复现", "",
                  "manifest.json 保留原 brief、解析后默认值、显式选择事实、来源路径与 SHA-256。下列快照保留读取时的原始字节，未知可选字段不丢失。",
                  "同一输入与编译器生成相同 job ID 和批次摘要；仅更换输出目录不会改变来源摘要。", ""])
    for key, source in manifest["sources"].items():
        lines.append(f"- {key}: [{source['snapshot']}]({source['snapshot']}) · SHA-256 `{source['sha256']}`")
    lines.extend(["", "## 原始编排与显式绘制内容", "",
                  "原始意图、场景与行为约束不发送给绘图模型。请逐项核对 render 白名单是否准确保留要显示的文字、数据、状态；unknowns 只作审阅，不自动补入画面。", ""])
    lines.extend(json_fence(manifest["brief"]))
    lines.extend(["", "## 选中目录的来源与检查责任", "",
                  "以下完整记录保留 evidence、motion、媒介 constraints 和格式 checks。仅合同列出的绘制字段进入 prompt；生成后检查及执行依赖由执行器另行消费。", ""])
    lines.extend(json_fence(manifest["selected_catalogue"]))
    lines.extend(["", "## 尚未验证", "",
                  "未调用模型或网络；没有生成回执、图像质量、透明 alpha、供应商尺寸支持、交互、代码或真实消费证据。",
                  "目录 evidence 是来源声明，工具不将种子配方升级为已验证风格。生成后的比较仍需检查适用场景、结构差异、可读性、素材裁切与实际背景。"])
    return "\n".join(lines) + "\n"


def json_fence(value):
    content = json_bytes(value).decode("utf-8").rstrip()
    fence = "`" * (max((len(run) for run in re.findall(r"`+", content)), default=2) + 1)
    return [fence + "json", content, fence]


def write_new_batch(output, payloads):
    """Reserve a new local directory exclusively; never merge into existing content."""
    created_files, created_dirs = [], []
    try:
        output.mkdir()
    except FileExistsError:
        raise InputError(f"{output}: already exists; choose a new output directory") from None
    try:
        for relative, raw in payloads.items():
            destination = output / relative
            for parent in reversed(destination.parent.relative_to(output).parents):
                folder = output / parent
                if folder != output and folder not in created_dirs:
                    folder.mkdir()
                    created_dirs.append(folder)
            folder = destination.parent
            if folder != output and folder not in created_dirs:
                folder.mkdir()
                created_dirs.append(folder)
            with destination.open("xb") as stream:
                created_files.append(destination)
                stream.write(raw)
    except BaseException:
        # Remove only files we created. Unexpected concurrent content is left alone.
        for path in list(reversed(created_files)) + list(reversed(created_dirs)):
            try:
                path.rmdir() if path.is_dir() else path.unlink()
            except OSError:
                pass
        try:
            output.rmdir()
        except OSError:
            pass
        raise


def print_catalogue(catalogues):
    print("目录 / 文字配方；没有生成或验证新图。\n")
    for kind, items in catalogues.items():
        print(f"{kind} ({len(items)})")
        for item in items.values():
            if kind == "formats":
                print(f"  {item['id']} · {item['name']} · {item['kind']} · {item['width']} × {item['height']} px")
            else:
                print(f"  {item['id']} · {item['name']} — {item['summary']}")
                print(f"    用语：{'、'.join(item['aliases'])}; 证据：{item['evidence']}")
                print(f"    适用：{'；'.join(item['suitable_for'])}; 避免：{'；'.join(item['avoid_for'])}")
        print()


def suggest(text, styles):
    text_value(text, "TEXT")
    query = text.casefold()
    matches = []
    for style in styles.values():
        vocabulary = [style["id"], style["name"]] + style["aliases"]
        terms = sorted({term for term in vocabulary if term.casefold() in query})
        if terms:
            matches.append((style, terms))
    return sorted(matches, key=lambda match: (-len(match[1]), match[0]["id"]))


def parser():
    root = argparse.ArgumentParser(description="Browse local recipes and prepare image jobs; no provider calls.",
                                   epilog="Examples: catalog | suggest '清爽' | prepare brief.json --out ./batch-01. Guide: ../references/tool.md beside this script.")
    root.add_argument("--catalog-dir", type=Path, default=DEFAULT_CATALOG, metavar="DIRECTORY",
                      help="local catalogue directory (default: catalog directory beside this Skill)")
    commands = root.add_subparsers(dest="command")
    usage_note = "Global --catalog-dir DIRECTORY goes before the command. No ENV/config. Guide: ../references/tool.md beside this script."
    commands.add_parser("catalog", help="browse style, medium and format recipes", epilog=usage_note)
    search = commands.add_parser("suggest", help="explain literal ID/name/alias matches; no aesthetic inference", epilog=usage_note)
    search.add_argument("text", metavar="TEXT", help="ordinary language to match against catalogue terms")
    prepare = commands.add_parser("prepare", help="validate and compile a brief into a new directory", epilog=usage_note)
    prepare.add_argument("brief", type=Path, metavar="BRIEF", help="UTF-8 JSON brief with explicit render content (contract v2)")
    prepare.add_argument("--out", type=Path, required=True, metavar="DIRECTORY",
                         help="new local output directory; parent must exist; never overwritten")
    return root


def main(argv=None):
    root = parser()
    args = root.parse_args(argv)
    if args.command is None:
        root.print_help()
        return 0
    try:
        catalogues, sources, snapshots = load_catalogues(args.catalog_dir)
        if args.command == "catalog":
            print_catalogue(catalogues)
        elif args.command == "suggest":
            matches = suggest(args.text, catalogues["styles"])
            print("仅匹配目录 ID、名称和 aliases 原词；不理解否定、同义或审美适合度。")
            if not matches:
                print("没有词汇匹配。请运行 catalog 浏览方向，或使用目录列出的用语。")
            for style, terms in matches:
                print(f"{style['id']} · {style['name']} — 命中：{'、'.join(terms)}")
                print(f"  {style['summary']}；避免：{'；'.join(style['avoid_for'])}；证据：{style['evidence']}")
        else:
            require(not os.path.lexists(args.out), args.out, "already exists; choose a new output directory")
            require(args.out.parent.is_dir(), args.out.parent, "output parent must be an existing directory")
            brief, raw = read_json(args.brief)
            sources["brief"] = source_record(args.brief, raw, "sources/brief.json")
            snapshots["sources/brief.json"] = raw
            compiler = Path(__file__).resolve()
            compiler_raw = compiler.read_bytes()
            sources["compiler"] = source_record(compiler, compiler_raw, "sources/design_batch.py")
            snapshots["sources/design_batch.py"] = compiler_raw
            manifest, payloads = plan_batch(brief, catalogues, sources, snapshots)
            write_new_batch(args.out, payloads)
            print(f"Prepared {len(manifest['jobs'])} jobs (no images generated): {args.out / 'review.md'}")
        return 0
    except (InputError, OSError, UnicodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
