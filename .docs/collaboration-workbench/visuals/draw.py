"""Author source for the revised resource-oriented SVG offering.

Python standard library only. Rows are illustrative bounded work records; source
groups project original-reference paths rather than introduce planning entities.
"""
from html import escape
from pathlib import Path

STYLE = """
text{font-family:system-ui,-apple-system,'Noto Sans CJK SC','Microsoft YaHei',sans-serif;font-size:14px;fill:#253542}
.paper{fill:#fff}.canvas{fill:#f5f7f9}.nav{fill:#f3f5f7}.wash{fill:#f8fafb}.selected{fill:#eaf3f0}
.line{fill:none;stroke:#e0e6eb;stroke-width:1}.border{fill:#fff;stroke:#ced8df;stroke-width:1}
.accent{fill:#176b5b}.green{fill:#176b5b}.amber{fill:#8e610d}.red{fill:#b04c45}.blue{fill:#45649a}
.green-bg{fill:#eaf3f0}.amber-bg{fill:#fff5df}.red-bg{fill:#fbeeed}.blue-bg{fill:#edf1f9}
.muted{fill:#697a88}.quiet{fill:#82919c}.white{fill:#fff}.small{font-size:12px}.caption{font-size:13px}
.medium{font-weight:600}.section{font-size:16px;font-weight:650}.title{font-size:21px;font-weight:650;letter-spacing:-.4px}.brand{font-size:18px;font-weight:750}
.mono{font-family:ui-monospace,'SFMono-Regular',Consolas,monospace}.stroke{fill:none;stroke:#6b7c89;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}
.green-stroke{fill:none;stroke:#176b5b;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}
.white-stroke{fill:none;stroke:#fff;stroke-width:1.6;stroke-linecap:round;stroke-linejoin:round}
.selected-border{fill:#f4f9f7;stroke:#176b5b;stroke-width:1.3}.shadow-line{fill:none;stroke:#cbd5dd;stroke-width:1}
.disabled{opacity:.4}
"""


def t(x, y, value, cls="", **attrs):
    extra = " ".join(f'{k.replace("_", "-")}="{escape(str(v), quote=True)}"' for k, v in attrs.items())
    return f'<text x="{x}" y="{y}" class="{cls}" {extra}>{escape(value)}</text>'


def r(x, y, width, height, cls="paper", radius=0):
    return f'<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="{radius}" class="{cls}"/>'


def l(x, y, end_x, end_y=None, cls="line"):
    return f'<path d="M{x} {y}L{end_x} {y if end_y is None else end_y}" class="{cls}"/>'


def dot(x, y, cls="accent", size=3):
    return f'<circle cx="{x}" cy="{y}" r="{size}" class="{cls}"/>'


def g(name, parts):
    return '<g id="' + name + '">\n' + "\n".join(parts) + "\n</g>"


def ic(x, y, kind, cls="stroke"):
    paths = {
        "search": "M12 12l5 5M13 7.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0",
        "chevron": "M4 6l4 4 4-4", "right": "M6 3l5 5-5 5", "plus": "M8 2v12M2 8h12",
        "close": "M3 3l10 10M13 3 3 13", "arrow": "M2 8h12M9 3l5 5-5 5",
        "file": "M3 1h7l4 4v12H3zM10 1v5h4M6 10h5M6 13h5",
        "folder": "M1 5h6l2-3h7v13H1z", "check": "M2 8l4 4 8-9",
        "shield": "M8 1 15 4v5c0 4-7 8-7 8S1 13 1 9V4zM4 8l3 3 5-6",
        "chat": "M2 2h13v10H8l-5 4v-4H2zM5 6h7M5 9h4",
        "link": "M6 5l2-2a4 4 0 0 1 6 6l-2 2M10 11l-2 2a4 4 0 0 1-6-6l2-2M5 10l6-6",
        "more": "M2 8h.1M8 8h.1M14 8h.1", "info": "M8 7v6M8 4h.1M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0",
        "edit": "M2 11l9-9 4 4-9 9H2zM9 4l4 4", "person": "M11 5a3 3 0 1 1-6 0 3 3 0 0 1 6 0M2 16v-2a6 6 0 0 1 12 0v2",
        "menu": "M2 3h13M2 8h13M2 13h13", "refresh": "M14 5a6 6 0 1 0 0 7M14 1v5H9",
    }
    return f'<path d="{paths[kind]}" transform="translate({x} {y})" class="{cls}"/>'


def control(x, y, name, kind, size=32, disabled=False):
    state = ' class="disabled" aria-disabled="true"' if disabled else ""
    return f'<g aria-label="{escape(name)}"{state}><title>{escape(name)}</title>' + r(x, y, size, size, "paper", 5) + ic(x + (size - 16) / 2, y + (size - 16) / 2, kind) + "</g>"


def button(x, y, width, label, primary=False, height=32):
    return r(x, y, width, height, "accent" if primary else "border", 5) + t(x + width / 2, y + height / 2 + 5, label, "medium white" if primary else "medium", text_anchor="middle")


def badge(x, y, label, width, tone="green"):
    return r(x, y, width, 24, tone + "-bg", 4) + t(x + width / 2, y + 17, label, "small medium " + tone, text_anchor="middle")


def av(x, y, name, size=24):
    tone = "blue" if name in {"Moss", "Ash"} else "green"
    return dot(x + size / 2, y + size / 2, tone + "-bg", size / 2) + t(x + size / 2, y + size / 2 + 4, name[:1], "small medium " + tone, text_anchor="middle")


def actor(x, y, name):
    return control(x - 3, y - 13, "安排参与者", "plus") if not name else av(x, y - 12, name) + t(x + 32, y + 5, name, "caption medium")


WORKS = [
    ("024", "为列表命令增加 CSV 导出", "Lin", "Moss", "—", "—", "等你 · Q1", "amber", "答复"),
    ("023", "修复配置文件路径解析", "Iris", "Moss", "r1", "检查中", "等待报告", "muted", ""),
    ("021", "统一命令行错误信息", "Lin", "Moss", "r1", "通过 · C1", "已接受 r1", "green", ""),
    ("019", "补齐 Shell 自动补全", "Iris", "Ash", "r1", "未请求", "待检查", "muted", "请求检查"),
    ("draft", "安装说明（本页草稿）", "", "", "—", "—", "未保存", "amber", "继续"),
    ("020", "修复 Windows 中文输出", "Lin", "Ash", "r2", "失败 · C2", "需修改", "red", "要求修改"),
]


def chrome():
    p = [r(0, 0, 1440, 900, "canvas"), r(0, 0, 1440, 48), l(0, 48, 1440), r(0, 49, 216, 827, "nav"), l(216, 48, 216, 876)]
    p += [r(18, 13, 22, 22, "accent", 5), t(29, 29, "A", "medium white", text_anchor="middle"), t(49, 31, "AgentX", "brand"), t(242, 30, "atlas-cli", "medium"), t(330, 30, "/ 资源工作区", "caption muted"), t(1103, 30, "设计示例", "small muted"), badge(1230, 12, "协作可用", 100), control(1354, 8, "查看固定项目与协作能力", "info")]
    nav = [t(20, 84, "资源", "small medium muted"), r(10, 100, 196, 36, "selected", 5), ic(21, 111, "folder", "green-stroke"), t(47, 123, "全部工作", "medium green"), t(188, 123, "5", "caption green", text_anchor="end")]
    nav += [t(20, 169, "交付 / 来源", "small medium muted"), control(172, 150, "来源分组按已有任务引用路径呈现", "info")]
    nav += [ic(22, 192, "folder"), t(48, 204, "cli-1.4", "medium"), t(187, 204, "4", "caption muted", text_anchor="end"), t(48, 228, ".tasks/cli-1.4/", "small muted mono")]
    nav += [ic(22, 253, "folder"), t(48, 265, "install", "medium"), t(187, 265, "1", "caption muted", text_anchor="end"), t(48, 289, ".tasks/install/", "small muted mono")]
    nav += [l(20, 328, 196), t(20, 365, "资源视图", "small medium muted"), ic(22, 386, "person"), t(49, 400, "Agents", "caption"), t(187, 400, "4", "small muted", text_anchor="end"), ic(22, 424, "file"), t(49, 438, "产物 / 结果", "caption"), ic(22, 462, "shield"), t(49, 476, "检查与接受", "caption")]
    nav += [l(20, 517, 196), t(20, 550, "Agents", "small medium muted")]
    for i, (name, status, tone) in enumerate([("Lin", "空闲", "muted"), ("Moss", "空闲", "muted"), ("Iris", "工作中", "green"), ("Ash", "工作中", "green")]):
        y = 574 + i * 49
        nav += [av(20, y, name, 28), t(59, y + 18, name, "caption medium"), dot(147, y + 14, tone), t(191, y + 18, status, "small " + tone, text_anchor="end")]
    nav += [l(20, 805, 196), control(17, 823, "打开当前项目连接与诊断", "info"), t(58, 844, "连接与诊断", "caption muted")]
    p.append(g("source-reference-navigation-and-agent-resources", nav))
    p += [r(216, 49, 1224, 827), t(240, 88, "全部工作", "title"), t(345, 87, "已载入 5 项", "small muted"), r(1010, 65, 230, 32, "border", 5), ic(1022, 73, "search"), t(1047, 87, "搜索任务、Agent、产物", "caption muted"), button(1252, 65, 164, "＋ 新建任务")]
    p += [badge(240, 109, "全部", 56), t(318, 126, "待我处理", "caption muted"), t(418, 126, "待检查", "caption muted"), t(516, 126, "进行中", "caption muted"), t(1180, 126, "按来源分组", "caption muted"), control(1273, 105, "更改来源分组与筛选", "chevron"), control(1311, 105, "刷新已载入工作", "refresh"), control(1349, 105, "列显示设置", "more")]
    p += [r(216, 876, 1224, 24, "wash"), l(0, 876, 1440), t(20, 892, "atlas-cli", "small muted"), t(240, 892, "5 项已载入 · 1 本页草稿", "small muted"), t(1416, 892, "更新于 10:42", "small muted", text_anchor="end")]
    return p


def matrix(mode="question"):
    p = [r(236, 145, 1180, 34, "wash"), t(248, 167, "任务", "small medium muted"), t(598, 167, "实现者", "small medium muted"), t(714, 167, "独立检查者", "small medium muted"), t(830, 167, "结果", "small medium muted"), t(926, 167, "检查", "small medium muted"), t(1090, 167, "待办 / 接受", "small medium muted"), t(1281, 167, "操作", "small medium muted")]
    for y, label, path, count in [(179, "cli-1.4", ".tasks/cli-1.4/", "4"), (403, "install", ".tasks/install/", "1")]:
        p += [r(236, y, 1180, 32, "wash"), ic(245, y + 8, "chevron"), ic(271, y + 8, "folder"), t(298, y + 22, label, "caption medium"), t(390, y + 22, path, "small muted mono"), t(1398, y + 22, count + " 项", "small muted", text_anchor="end")]
    for i, entry in enumerate(WORKS):
        key, title, impl, checker, result, check, state, tone, action = entry
        y = 211 + i * 48 if i < 4 else 483 if key == "draft" else 435
        if key == "024":
            p += [r(236, y, 1180, 48, "selected"), r(236, y + 5, 3, 38, "accent", 1)]
            if mode == "assignment":
                state, tone, action = "待派发", "muted", "派发"
            elif mode == "result":
                result, check, state, tone, action = "r2", "通过 · C2", "待接受 r2", "green", ""
        if key == "draft":
            p += [r(236, y + 2, 1180, 44, "wash", 4), ic(250, y + 14, "edit"), t(282, y + 29, "本页草稿 · 安装说明", "caption muted"), t(1090, y + 29, "未保存", "caption amber"), button(1193, y + 8, 74, "继续"), control(1358, y + 8, "放弃本页草稿", "close")]
            continue
        p += [t(250, y + 29, "#" + key, "small muted mono"), t(299, y + 29, title, "caption medium" if key == "024" else "caption")]
        p += [actor(598, y + 23, impl), actor(714, y + 23, checker), t(834, y + 29, result, "caption blue mono" if result != "—" else "caption quiet")]
        check_tone = "red" if check.startswith("失败") else "green" if check.startswith("通过") else "muted"
        p += [ic(926, y + 15, "shield", "green-stroke" if check_tone == "green" else "stroke") if check not in {"—", "未请求"} else "", t(949 if check not in {"—", "未请求"} else 929, y + 29, check, "caption " + check_tone)]
        if action:
            p += [dot(1094, y + 23, tone), t(1106, y + 29, state, "caption " + tone), button(1193, y + 8, 74, action) if len(action) <= 2 else button(1173, y + 8, 94, action)]
        else:
            p += [t(1090, y + 29, state, "caption " + tone)]
        note_name = "工作已接受，不能继续发送补充" if state.startswith("已接受") else "给 #" + key + " 的参与者发送补充" if impl else "暂无接收者；先安排参与者"
        p += [control(1282, y + 8, "查看 #" + key + " 的原始来源与执行依据", "file"), control(1320, y + 8, note_name, "chat", disabled=not impl or state.startswith("已接受")), control(1358, y + 8, "#" + key + " 更多操作", "more"), l(236, y + 48, 1416)]
    return g("source-grouped-work-agent-result-check-matrix", p)


def panel_header(mode="question"):
    p = [l(216, 548, 1440, cls="shadow-line"), r(216, 549, 1224, 49, "wash"), t(240, 579, "#024", "caption muted mono"), t(294, 579, "CSV 导出", "section")]
    active = "产物" if mode == "result" else "任务"
    for x, label in [(500, "任务"), (590, "产物"), (680, "检查"), (770, "消息")]:
        p += [t(x, 579, label, "caption medium green" if label == active else "caption muted")]
        if label == active:
            p += [r(x, 594, 29, 3, "accent", 1)]
    if mode == "assignment":
        p += [badge(1014, 562, "已保存 · 待派发", 164)]
    p += [control(1299, 557, "编辑 #024 目标、条件与角色", "edit"), control(1337, 557, "复制 #024 工作链接", "link"), control(1375, 557, "收起所选任务详情", "close"), l(216, 598, 1440)]
    return g("persistent-selected-task-and-local-tabs", p)


def question_detail():
    p = [t(240, 628, "目标", "small medium muted"), t(240, 655, "列表命令支持 CSV 导出，", "caption"), t(240, 679, "中文、引号与换行能正确读取。", "caption"), t(240, 714, "验收条件 · b1", "small medium muted")]
    for i, label in enumerate(["固定表头与 UTF-8", "按 CSV 规则转义特殊字符", "不静默覆盖已有文件"]):
        p += [t(240, 742 + i * 26, str(i + 1).zfill(2), "small quiet mono"), t(270, 742 + i * 26, label, "caption")]
    p += [ic(241, 825, "file"), t(267, 839, "csv-export.md", "caption blue mono"), control(487, 816, "查看原始任务来源及保存的依据", "info"), l(548, 617, 548, 856)]
    p += [av(573, 615, "Lin"), t(607, 633, "Lin", "caption medium"), t(651, 633, "Q1 · 覆盖策略", "caption muted"), t(1398, 633, "10:42", "small muted", text_anchor="end")]
    p += [t(574, 668, "导出目标已存在时，是否允许用 --force 显式覆盖？", "section")]
    p += [r(574, 687, 370, 36, "border", 5), dot(591, 705, "line", 6), t(610, 710, "始终报错，不允许覆盖", "caption"), r(958, 687, 440, 36, "selected-border", 5), dot(976, 705, "green-stroke", 6), dot(976, 705, "green", 2.5), t(994, 710, "显式 --force 才覆盖", "caption medium green")]
    p += [control(572, 735, "展开自定义答复输入", "edit"), t(612, 757, "自定义答复", "caption blue"), button(1274, 737, 124, "回复 Lin", True), l(574, 787, 1398)]
    p += [r(574, 809, 824, 36, "border", 5), t(588, 833, "补充给 Lin…", "caption muted"), control(1359, 811, "发送普通补充，不改变问题状态", "arrow")]
    return g("compact-goal-criteria-and-question-answer", p)


def assignment_detail():
    p = [t(240, 626, "标题", "small medium muted"), r(240, 637, 372, 34, "border", 5), t(252, 660, "为列表命令增加 CSV 导出", "caption")]
    p += [t(240, 698, "目标", "small medium muted"), r(240, 709, 372, 62, "border", 5), t(252, 734, "CSV 导出支持中文、引号与换行；", "caption"), t(252, 757, "明确已有文件的覆盖策略。", "caption")]
    p += [t(240, 802, "来源", "small medium muted"), r(286, 783, 326, 34, "border", 5), t(298, 806, ".tasks/cli-1.4/csv-export.md", "small mono")]
    p += [t(642, 626, "验收条件 · b1", "small medium muted"), r(642, 637, 380, 134, "border", 5)]
    for i, label in enumerate(["固定表头与 UTF-8 编码", "中文、引号、换行按 CSV 规则转义", "不静默覆盖已有文件"]):
        p += [t(655, 666 + i * 41, str(i + 1).zfill(2), "small muted mono"), t(683, 666 + i * 41, label, "caption")]
    for y, label, name in [(626, "实现者", "Lin"), (716, "独立检查者", "Moss")]:
        p += [t(1052, y, label, "small medium muted"), r(1052, y + 11, 346, 52, "border", 5), av(1065, y + 25, name, 26), t(1103, y + 44, name, "medium"), t(1246, y + 44, "…a71c" if name == "Lin" else "…c82f", "small mono muted"), ic(1367, y + 29, "chevron")]
    p += [l(236, 831, 1416), control(240, 836, "保存和派发分别留存；当前已保存，未发消息", "info"), button(1107, 838, 112, "关闭"), button(1231, 838, 167, "派发给 Lin", True)]
    return g("inline-work-and-role-editor-saved-before-dispatch", p)


def result_detail():
    p = [t(240, 627, "r2", "section blue mono"), t(281, 627, "Lin", "caption medium"), t(339, 627, "git:3e81c42 · b1", "small mono muted"), control(691, 607, "实现者上报来源；检查者报告观察到相同指纹", "info"), t(240, 656, "CSV 导出、转义与显式覆盖已实现。", "caption")]
    for i, (path, detail) in enumerate([("src/commands/list.rs", "命令参数"), ("src/output/csv.rs", "编码与转义"), ("tests/csv_export.rs", "导出与覆盖案例")]):
        y = 681 + i * 36
        p += [ic(242, y - 12, "file"), t(270, y + 2, path, "caption blue mono"), t(611, y + 2, detail, "small muted"), control(724, y - 19, "查看 " + path + " 的保存引用", "link"), l(240, y + 14, 764)]
    p += [l(795, 615, 795, 789), t(818, 627, "Moss · C2", "section"), t(960, 627, "r2 / b1", "small mono muted"), badge(1280, 609, "上报通过", 118)]
    for i, (label, evidence) in enumerate([("表头与 UTF-8", "中文读回"), ("特殊字符转义", "逗号 / 引号 / 多行"), ("不静默覆盖", "无 --force 时拒绝")]):
        y = 666 + i * 36
        p += [ic(820, y - 11, "check", "green-stroke"), t(848, y + 2, label, "caption"), t(1398, y + 2, evidence, "small muted", text_anchor="end"), l(818, y + 14, 1398)]
    p += [ic(820, 770, "file"), t(847, 784, "checks/csv-r2.md", "caption blue mono"), control(1322, 764, "查看 C2 完整报告及所观察来源", "link"), control(1360, 764, "重新检查 r2，需填写理由", "refresh")]
    p += [r(236, 808, 1180, 61, "selected", 5), t(252, 832, "接受 r2 · 检查 C2 · 条件 b1", "caption medium"), t(252, 854, "记录接受，不执行合并或发布。", "small muted"), button(1078, 823, 92, "取消"), button(1182, 823, 216, "确认接受 r2", True)]
    return g("result-artifact-check-relationships-and-local-acceptance", p)


def desktop(mode="question"):
    detail = {"question": question_detail, "assignment": assignment_detail, "result": result_detail}[mode]
    return chrome() + [matrix(mode), panel_header(mode), detail()]


def narrow():
    p = [r(0, 0, 390, 844), r(0, 0, 390, 48, "nav"), control(8, 2, "展开交付来源与资源导航", "menu", 44), t(62, 31, "atlas-cli", "medium"), badge(268, 12, "协作可用", 106)]
    p += [t(16, 79, "全部工作", "section"), t(110, 79, "5 项", "small muted"), t(169, 79, "草稿 1", "small amber"), control(272, 52, "搜索当前已载入资源", "search", 44), control(328, 52, "创建任务", "plus", 44), r(0, 98, 390, 34, "wash"), ic(16, 108, "chevron"), ic(42, 107, "folder"), t(68, 121, "cli-1.4", "caption medium"), t(374, 121, "4 项", "small muted", text_anchor="end")]
    for i, (key, title, roles, state, tone) in enumerate([("024", "CSV 导出", "Lin → Moss", "Q1 等你", "amber"), ("023", "配置路径解析", "Iris → Moss", "r1 检查中", "muted"), ("021", "命令行错误信息", "Lin → Moss", "r1 已接受", "green"), ("019", "Shell 自动补全", "Iris → Ash", "r1 待检查", "muted")]):
        y = 132 + i * 48
        p += [r(0, y, 390, 48, "selected" if i == 0 else "paper"), t(16, y + 20, "#" + key, "small mono muted"), t(65, y + 20, title, "caption medium"), t(65, y + 40, roles, "small muted mono"), t(290, y + 29, state, "small " + tone, text_anchor="end"), control(330, y + 2, "打开 #" + key + " 的上下文操作", "more", 44), l(16, y + 48, 374)]
    p += [r(0, 324, 390, 40, "wash"), ic(17, 335, "right"), ic(43, 335, "folder"), t(69, 349, "install", "caption medium"), t(374, 349, "1 项", "small muted", text_anchor="end")]
    p += [l(0, 378, 390, cls="shadow-line"), r(0, 379, 390, 48, "wash"), t(16, 409, "#024 · CSV 导出", "section"), control(326, 381, "收起任务详情，保留资源选择", "close", 44)]
    for x, label in [(16, "任务"), (108, "产物"), (200, "检查"), (292, "消息")]:
        p += [t(x, 454, label, "medium green" if x == 16 else "muted")]
    p += [r(16, 469, 28, 3, "accent", 1), l(0, 472, 390), av(16, 490, "Lin"), t(51, 509, "Lin · Q1", "caption medium"), t(374, 509, "覆盖策略", "caption muted", text_anchor="end")]
    p += [t(16, 545, "目标文件已存在时，是否允许", "section"), t(16, 570, "用 --force 显式覆盖？", "section"), r(16, 590, 358, 44, "border", 5), dot(35, 612, "line", 6), t(55, 617, "始终报错，不允许覆盖", "caption"), r(16, 646, 358, 44, "selected-border", 5), dot(35, 668, "green-stroke", 6), dot(35, 668, "green", 2.5), t(55, 673, "显式 --force 才覆盖", "caption medium green")]
    p += [control(8, 700, "展开自定义答复", "edit", 44), t(61, 727, "自定义答复", "caption blue"), button(234, 700, 140, "回复 Lin", True, 44), l(16, 759, 374), control(8, 768, "查看任务目标、条件和原始来源", "file", 44), control(62, 768, "查看参与者分工与身份", "person", 44), r(122, 770, 252, 40, "border", 5), t(134, 796, "补充给 Lin…", "caption muted"), t(195, 837, "设计示例", "small muted", text_anchor="middle")]
    return [g("narrow-source-task-result-matrix-and-local-question", p)]


def write(filename, title, width, height, content):
    description = "资源工作区修订稿；来源、任务、分工、产物与检查关联可见，选中对象的操作在局部展开。全部内容为设计示例。"
    doc = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title description">', f'<title id="title">{escape(title)}</title>', f'<desc id="description">{description}</desc>', f'<defs><style>{STYLE}</style></defs>'] + content + ["</svg>"]
    Path(__file__).with_name(filename).write_text("\n".join(doc) + "\n", encoding="utf-8")


if __name__ == "__main__":
    write("01-workbench.svg", "AgentX 资源工作区：任务、分工、产物与待办", 1440, 900, desktop())
    write("02-assignment.svg", "AgentX 资源工作区：局部安排与显式派发", 1440, 900, desktop("assignment"))
    write("03-result-acceptance.svg", "AgentX 资源工作区：同源结果、独立检查与接受", 1440, 900, desktop("result"))
    write("04-narrow.svg", "AgentX 窄屏资源工作区：保留工作关系与局部答复", 390, 844, narrow())
