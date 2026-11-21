"""Detail scenes for the accepted resource workspace; uses draw.py primitives.

These are finite illustrative operation states, not executable product/UI code.
Run after draw.py. Generated previews remain controller-owned.
"""
from pathlib import Path
import draw as d

r, t, l, ic, g = d.r, d.t, d.l, d.ic, d.g
button, badge, control, av = d.button, d.badge, d.control, d.av


def workspace_view(title, caption="已载入工作关联"):
    p = d.chrome()
    p += [r(10,100,196,36,"nav",5),ic(21,111,"folder"),t(47,123,"全部工作","caption"),t(188,123,"5","small muted",text_anchor="end")]
    nav_y,nav_name,nav_icon = (380,"Agents","person") if title=="Agents" else (418,"产物 / 结果","file") if title=="产物 / 结果" else (456,"检查与接受","shield")
    p += [r(10,nav_y,196,36,"selected",5),ic(22,nav_y+6,nav_icon,"green-stroke"),t(49,nav_y+20,nav_name,"caption medium green")]
    p += [r(218, 50, 1222, 824), t(240, 87, title, "title"), t(420, 86, caption, "small muted"), r(1010, 65, 230, 32, "border", 5), ic(1021, 72, "search"), t(1048, 87, "搜索当前已载入资源", "caption muted"), control(1253, 65, "刷新当前资源", "refresh"), control(1291, 65, "当前视图筛选", "more")]
    return p


def head(y, values):
    p = [r(236, y, 1180, 34, "wash")]
    for x, label in values:
        p += [t(x, y + 22, label, "small medium muted")]
    return p


def agents():
    p = workspace_view("Agents", "当前项目 · 4 个已运行身份")
    p += [badge(240, 112, "全部", 56), t(318, 129, "有已载入工作", "caption muted"), t(475, 129, "可选状态", "caption muted")]
    p += head(151, [(250,"Agent / 精确身份"),(565,"运行观测"),(740,"可选状态"),(924,"已载入关联工作"),(1310,"操作")])
    for i, (name, ref, runtime, relation) in enumerate([("Lin","…a71c","空闲","#024 实现 / #021 实现 / #020 实现"),("Moss","…c82f","空闲","#024 检查 / #023 检查 / #021 检查"),("Iris","…e93d","工作中","#023 实现 / #019 实现"),("Ash","…f04a","工作中","#019 检查 / #020 检查")]):
        y = 185 + i * 54
        p += [r(236,y,1180,54,"selected" if name=="Moss" else "paper"),av(250,y+13,name,28),t(291,y+25,name,"medium"),t(291,y+44,ref,"small mono muted"),t(565,y+31,runtime,"caption"),badge(740,y+15,"可接收工作",110),t(924,y+31,relation,"caption"),control(1320,y+11,"查看 "+name+" 精确身份与关联工作","person"),control(1358,y+11,"复制 "+name+" 参与者引用","link"),l(236,y+54,1416)]
    p += [l(216,425,1440),r(216,426,1224,48,"wash"),av(240,438,"Moss"),t(277,457,"Moss", "section"),t(350,457,"atlas-cli · …c82f", "caption mono muted"),control(1370,433,"关闭 Agent 详情并返回选中行","close")]
    p += [t(240,510,"身份", "small medium muted"),t(240,540,"moss · incarnation …c82f", "caption mono"),control(485,518,"当前绑定引用精确身份；同名新身份不会自动替换","info"),t(240,570,"项目  atlas-cli", "caption"),t(240,600,"运行  空闲 · 10:42 观测", "caption"),t(240,630,"可选  可接收工作", "caption green"),control(483,610,"运行空闲与可接收工作是不同事实","info"),l(546,491,546,830)]
    p += [t(570,510,"关联工作（已载入）", "section"),t(1250,510,"来源 / cli-1.4", "caption muted")]
    for i,(task,role,state) in enumerate([("#024 CSV 导出","独立检查者","待结果"),("#023 配置路径解析","独立检查者","检查中 · r1"),("#021 命令行错误信息","独立检查者","已接受 r1")]):
        y=536+i*67
        p += [t(570,y+24,task,"caption blue"),t(904,y+24,role,"caption"),t(1100,y+24,state,"caption muted"),control(1355,y+6,"查看 "+task+"，保留 Agent 视图返回点","arrow"),l(570,y+48,1398)]
    p += [control(1310,676,"#021 已接受并关闭；保留阅读、来源、复制与新建关联工作","info"),button(1210,800,188,"查看 #024 任务")]
    return [g("agent-resource-table-and-exact-identity-related-work",p)]


def create():
    p=d.chrome()+[d.matrix(),r(216,325,1224,550),l(216,325,1440,cls="shadow-line"),r(216,326,1224,49,"wash"),t(240,357,"新任务 · 本页草稿", "section"),badge(443,337,"未保存",88,"amber"),control(1370,333,"收起编辑器，保留本页草稿","close")]
    p += [t(240,404,"标题 *", "small medium muted"),r(240,417,748,35,"border",5),t(253,441,"补齐安装说明与常见问题", "caption"),t(240,481,"目标 *", "small medium muted"),r(240,494,748,63,"border",5),t(253,520,"覆盖 macOS / Linux 的安装步骤，", "caption"),t(253,544,"补充配置路径和网络连接的常见问题。", "caption")]
    p += [t(240,588,"验收条件 *", "small medium muted"),r(240,601,748,112,"border",5),t(253,628,"01  从空目录按说明完成安装", "caption"),t(253,660,"02  文档中的命令和路径可复查", "caption"),control(944,670,"添加一项验收条件","plus"),t(240,746,"来源", "small medium muted"),r(286,727,702,35,"border",5),t(299,751,".tasks/install/guide.md", "caption mono")]
    p += [t(1020,404,"实现者 *", "small medium muted"),r(1020,417,378,48,"border",5),av(1033,429,"Lin"),t(1071,447,"Lin · …a71c", "caption medium"),ic(1368,433,"chevron"),t(1020,499,"独立检查者 *", "small medium muted"),r(1020,511,378,39,"invalid-field",5),t(1033,537,"选择已运行 Agent", "caption muted"),ic(1368,523,"chevron"),t(1020,575,"请选择独立检查者", "caption red")]
    # Open picker presents valid, same-identity, busy-valid, and stopped cases.
    p += [r(1004,585,410,234,"border",7),r(1016,597,386,32,"border",4),ic(1027,604,"search"),t(1053,619,"搜索当前项目候选", "caption muted")]
    for y,name,detail,tone,selectable in [(636,"Moss","…c82f · 可接收工作","green",True),(679,"Lin","…a71c · 与实现者相同","muted",False),(722,"Ash","…f04a · 工作中，可接收","green",True),(765,"Lin（旧身份）","…91ab · 已退出","muted",False)]:
        p += [r(1016,y,386,42,"paper"),av(1025,y+8,name,24),t(1061,y+17,name,"caption medium"),t(1061,y+35,detail,"small "+tone),control(1361,y+5,"选择 "+name if selectable else detail,"right" if selectable else "info",disabled=not selectable)]
    p += [l(236,829,1416),button(1073,838,150,"放弃草稿"),button(1235,838,163,"保存任务",True)]
    return [g("local-draft-validation-and-exact-agent-picker",p)]


def messages():
    p=d.chrome()+[d.matrix(),d.panel_header(),r(237,600,1178,274)]
    p += [r(1090,212,182,46,"selected"),t(1090,240,"已答 Q1 · 等待更新","caption muted")]
    # Selected local perspective, without changing the work.
    p += [r(489,557,351,40,"wash"),t(500,579,"任务","caption muted"),t(590,579,"产物","caption muted"),t(680,579,"检查","caption muted"),t(770,579,"消息","caption medium green"),r(770,594,29,3,"accent",1)]
    p += [badge(240,610,"全部",56),t(319,627,"问题 / 答复", "caption muted"),t(453,627,"补充", "caption muted"),t(550,627,"派发", "caption muted")]
    p += [av(240,649,"Lin"),t(275,666,"Q1 · 你 → Lin（实现者）", "caption medium"),t(275,693,"显式 --force 才覆盖；默认拒绝覆盖。", "caption"),badge(789,647,"已答复",88),badge(891,647,"送达未知",104,"amber"),control(1008,643,"查看此答复的原生消息接收与外部投递证据","info"),l(240,716,1044)]
    p += [r(240,734,804,48,"amber-bg",5),t(253,754,"普通补充结果待确认 · cmd …7b20", "caption amber"),t(253,774,"接收者：Moss（独立检查者）", "small muted"),button(904,742,128,"查询原操作")]
    p += [button(240,800,233,"检查者 Moss  ▾"),r(485,800,559,44,"border",5),t(498,826,"请在检查报告中注明覆盖策略案例。", "caption"),control(1005,806,"上次补充结果待确认；暂不重复发送","arrow",disabled=True)]
    p += [l(1070,615,1070,854),t(1093,633,"答复回执", "section"),control(1360,613,"关闭回执并返回消息","close"),t(1093,669,"Q1 → Lin · 当前实现者", "caption"),t(1093,702,"消息  …8c14", "caption mono"),t(1093,735,"原生接收  已保存", "caption green"),t(1093,768,"外部投递  未知", "caption amber"),t(1093,801,"工作确认  尚未报告", "caption muted"),control(1090,819,"复制这条消息的稳定引用","link")]
    return [g("recipient-specific-note-question-and-command-recovery",p)]


def history():
    p=d.chrome()+[d.matrix("result"),d.panel_header("result"),r(237,600,1178,274)]
    p += [r(823,211,268,48,"selected"),t(834,240,"r1","caption mono blue"),ic(926,226,"shield"),t(949,240,"失败 · C1","caption red"),r(1090,212,182,46,"selected"),t(1090,240,"需修改","caption red")]
    p += [t(240,627,"结果 / 检查历史", "small medium muted"),r(240,645,170,61,"selected",5),t(252,668,"r1 · 当前", "caption medium"),t(252,693,"C1 检查失败", "small red"),t(252,744,"C0 · 历史检查", "caption blue"),t(252,770,"r1 无法检查", "small amber"),control(366,730,"查看同一 r1 的旧检查 C0，只读","right"),l(432,618,432,846)]
    p += [t(456,628,"C1 · Moss 上报失败", "section red"),t(761,628,"r1 / b1", "caption mono muted"),t(456,662,"01 表头与 UTF-8", "caption"),ic(787,649,"check","green-stroke"),t(456,697,"02 特殊字符转义", "caption"),ic(787,684,"check","green-stroke"),t(456,732,"03 不静默覆盖", "caption red"),ic(787,719,"close"),t(456,764,"已有文件被覆盖；预期拒绝。", "caption red"),ic(457,800,"file"),t(483,814,"checks/csv-r1.md", "caption mono blue"),control(792,795,"查看 C1 逐条件依据与来源","link"),l(858,618,858,846)]
    p += [t(883,628,"要求修改 r1", "section"),t(883,660,"发给 Lin · 实现者", "caption medium"),r(883,676,515,77,"border",5),t(896,702,"已有文件被静默覆盖，请默认拒绝覆盖，", "caption"),t(896,728,"仅在显式 --force 时允许覆盖。", "caption"),t(883,780,"理由 *", "small muted"),button(883,802,126,"取消"),button(1021,802,183,"发送修改要求",True),button(1216,802,182,"重新检查…")]
    return [g("current-failed-check-historical-unable-report-and-change-request",p)]


def results():
    p=workspace_view("产物 / 结果", "已载入工作中的报告引用")
    p += [badge(240,111,"当前结果",94),t(358,128,"含历史", "caption muted"),t(471,128,"未检查", "caption muted"),t(573,128,"检查失败", "caption muted")]
    p += head(150,[(250,"所属任务"),(490,"结果 / 作者"),(665,"不可变来源"),(868,"产物"),(1002,"检查"),(1193,"决定 / 操作")])
    entries=[("#024 CSV 导出","r2 · Lin","git:3e81c42","3 个引用","通过 · C2","待接受"),("#023 配置路径","r1 · Iris","git:7d24a11","2 个引用","检查中","—"),("#021 错误信息","r1 · Lin","git:a207be1","2 个引用","通过 · C1","已接受 r1"),("#019 Shell 补全","r1 · Iris","git:f36a522","1 个引用","未请求","请求检查"),("#020 中文输出","r2 · Lin","git:e1f6b93","2 个引用","失败 · C2","需修改")]
    for i,entry in enumerate(entries):
        y=184+i*48
        p += [r(236,y,1180,48,"selected" if i==0 else "paper")]
        for x,value,cls in zip([250,490,665,868,1002,1193],entry,["caption blue","caption medium","small mono muted","caption","caption","caption"]):
            p += [t(x,y+30,value,cls)]
        p += [control(1360,y+8,"查看 "+entry[0]+" 固定结果及其产物","right"),l(236,y+48,1416)]
    p += [l(216,454,1440),r(216,455,1224,48,"wash"),t(240,486,"#024 / r2 / src/output/csv.rs", "section"),control(1370,463,"返回结果表，保留所选产物","close")]
    p += [t(240,539,"保存的产物引用", "small medium muted"),t(240,571,"src/output/csv.rs", "caption mono blue"),t(240,603,"来源 git:3e81c42", "caption mono"),t(240,635,"Lin 上报 · 条件 b1", "caption"),t(240,667,"摘要：UTF-8 表头、引号与换行转义。", "caption"),button(240,705,156,"复制引用"),control(409,705,"此处展示报告引用，不读取当前文件内容","info"),l(777,521,777,836)]
    p += [t(802,539,"关联依据", "section"),t(802,578,"#024 为列表命令增加 CSV 导出", "caption blue"),t(802,617,"实现者 Lin  →  独立检查者 Moss", "caption"),badge(802,642,"C2 上报通过 · r2 / b1",234),t(802,704,"来源指纹一致（Moss 上报）", "caption green"),button(802,741,150,"查看检查 C2"),button(967,741,170,"返回任务 #024"),button(1152,741,246,"接受 r2…")]
    return [g("cross-work-result-view-and-attributed-artifact-reference",p)]


def checks():
    p=workspace_view("检查与接受", "当前 / 历史 · 仅已载入工作")
    p += [badge(240,112,"全部",56),t(318,129,"待报告","caption muted"),t(417,129,"失败 / 无法检查","caption muted"),t(590,129,"已接受","caption muted")]
    p += head(151,[(250,"任务 / 记录"),(531,"目标依据"),(733,"检查者 / 操作者"),(953,"结论"),(1169,"当前 / 历史")])
    rows=[("#024 / 检查 C2","r2 / b1","Moss","上报通过","当前，可接受"),("#024 / 检查 C1","r1 / b1","Moss","检查失败","历史"),("#023 / 请求 CR1","r1 / b1","Moss","等待报告","当前请求"),("#021 / 接受 D1","r1 / C1 / b1","操作者","已接受 r1","已关闭"),("#020 / 检查 C2","r2 / b1","Ash","检查失败","当前")]
    for i,(record,basis,author,verdict,current) in enumerate(rows):
        y=185+i*48
        p += [r(236,y,1180,48,"selected" if i==0 else "paper"),t(250,y+30,record,"caption medium"),t(531,y+30,basis,"caption mono"),t(733,y+30,author,"caption"),t(953,y+30,verdict,"caption red" if verdict=="检查失败" else "caption green" if "通过" in verdict or "接受" in verdict else "caption muted"),t(1169,y+30,current,"caption muted"),control(1360,y+8,"查看 "+record+" 的固定对象","right"),l(236,y+48,1416)]
    p += [l(216,454,1440),r(216,455,1224,48,"wash"),t(240,486,"#024 / C2 / r2 / b1","section"),control(1370,463,"收起检查详情，保留资源视图","close"),t(240,538,"Moss 上报通过","section green"),t(240,570,"观察来源 git:3e81c42 · 与 r2 一致","caption"),t(240,604,"01 表头/UTF-8   02 特殊字符转义   03 不静默覆盖","caption"),t(240,640,"报告 checks/csv-r2.md","caption mono blue"),button(240,672,172,"查看结果 r2"),button(428,672,172,"重新检查…"),l(805,521,805,836)]
    p += [t(831,538,"重新检查同一 r2","section"),t(831,570,"检查者 Moss · 新请求会替代旧检查资格","caption"),t(831,601,"理由 *","small medium muted"),r(831,614,567,80,"border",5),t(845,641,"请复查新环境下的 CSV 读回结果，","caption"),t(845,669,"保留相同 git:3e81c42 来源。","caption"),button(1008,724,110,"取消"),button(1130,724,268,"请求重新检查",True)]
    return [g("cross-work-check-decision-view-and-recheck-reason",p)]


def edit_conflict():
    p=d.chrome()+[d.matrix(),r(216,326,1224,549),l(216,326,1440),r(216,327,1224,48,"wash"),t(240,358,"#024 / 编辑任务依据","section"),badge(494,338,"未保存 · 本地 b1",159,"amber"),control(1370,334,"收起编辑，保留本地字段","close")]
    p += [r(240,394,1158,48,"amber-bg",5),t(253,423,"任务已更新到 b2；本地草稿仍保留。","caption amber"),button(1194,402,190,"基于更新继续编辑")]
    p += head(460,[(253,"字段"),(420,"你的草稿（基于 b1）"),(887,"服务器当前 b2")])
    for i,(field,local,remote) in enumerate([("目标","CSV 导出支持 --force 覆盖","CSV 导出保留默认拒绝覆盖"),("条件 03","显式 --force 才覆盖","覆盖需保存备份"),("来源",".tasks/cli-1.4/csv-export.md","相同保存引用")]):
        y=494+i*69
        p += [t(253,y+28,field,"caption medium"),t(420,y+28,local,"caption"),t(887,y+28,remote,"caption"),l(240,y+56,1398)]
    p += [t(240,748,"选择更新底稿后逐项确认保留内容，再显式保存。","caption muted"),button(976,786,190,"载入更新（放弃本地）"),button(1178,786,220,"继续保留草稿")]
    p += [r(487,687,467,132,"border",7),t(505,716,"放弃未保存更改？","section"),t(505,746,"只放弃本地字段，不撤回已派发工作。","caption muted"),button(505,768,192,"继续编辑"),button(709,768,225,"放弃更改")]
    return [g("existing-brief-conflict-field-comparison-and-discard-boundary",p)]


def narrow_picker():
    p=d.narrow()+[r(0,132,390,690),r(0,132,390,49,"wash"),t(16,163,"本页草稿 / 独立检查者","section"),control(330,135,"返回草稿，保留已填字段","close",44),r(16,195,358,44,"border",5),ic(29,209,"search"),t(57,223,"搜索当前项目候选","caption muted")]
    for i,(name,ref,status,ok) in enumerate([("Moss","…c82f","可接收工作",True),("Lin","…a71c","与实现者相同",False),("Ash","…f04a","工作中 · 可接收",True),("Lin（旧身份）","…91ab","已退出",False)]):
        y=255+i*72
        p += [r(16,y,358,64,"paper"),av(25,y+16,name,28),t(65,y+25,name,"caption medium"),t(65,y+49,ref+" · "+status,"small green" if ok else "small muted"),control(322,y+10,"选择 "+name if ok else status,"right" if ok else "info",44,disabled=not ok),l(16,y+64,374)]
    p += [r(0,751,390,70,"wash"),t(16,778,"当前实现者 Lin","caption muted"),t(16,804,"只选择候选，尚未保存或派发。","small muted")]
    return [g("narrow-candidate-sheet-preserves-draft-and-exact-identity",p)]


def sheet():
    p=[r(0,0,1440,1060),t(24,39,"组件与状态参考", "title"),t(1416,38,"设计示例", "small muted",text_anchor="end"),l(24,58,1416)]
    p += [t(24,90,"行、状态与焦点", "section"),t(24,120,"默认 / hover / selected / keyboard focus 彼此独立", "caption muted")]
    for i,(label,cls) in enumerate([("默认行", "paper"),("悬停行", "wash"),("选中行", "selected")]):
        y=141+i*48
        p += [r(24,y,660,44,cls,4),t(40,y+28,"#024  CSV 导出", "caption"),t(271,y+28,label,"caption muted"),badge(421,y+10,"等你 · Q1",99,"amber"),control(633,y+6,"答复 Q1","chat")]
    p += [r(630,242,38,38,"focus-ring",6),t(24,324,"表单与提交", "section"),t(24,355,"标题 *", "small medium muted"),r(24,368,660,36,"invalid-field",5),t(38,392,"", "caption"),t(24,425,"请输入任务标题", "caption red"),button(24,449,144,"保存任务",True),button(183,449,144,"保存中…",True),r(344,449,152,32,"wash",5),t(420,471,"派发（只读）", "caption muted",text_anchor="middle"),control(509,449,"当前监听仅允许读取，不能派发","info")]
    p += [t(24,524,"命令结果与恢复", "section"),r(24,544,660,61,"amber-bg",5),t(38,569,"结果待确认 · cmd …7b20", "caption amber"),t(38,591,"保留原输入与身份", "small muted"),button(521,558,148,"查询原操作"),r(24,621,660,52,"red-bg",5),t(38,653,"未保存：所选身份已失效", "caption red"),button(521,631,148,"更换角色")]
    p += [t(24,716,"并发更新 / 脏草稿", "section"),r(24,736,660,92,"amber-bg",5),t(38,761,"任务已更新 · 你的草稿仍保留", "caption amber"),button(38,780,158,"查看字段差异"),button(212,780,216,"基于更新继续编辑"),t(24,864,"关闭未保存编辑器", "caption medium"),button(24,880,186,"继续编辑"),button(226,880,186,"放弃更改")]
    p += [l(708,78,708,1028),t(736,90,"事实、检查与模式", "section")]
    for i,(label,tone) in enumerate([("Lin 上报结果 r2", "blue"),("Moss 上报通过 · C2", "green"),("检查失败 · 已覆盖文件", "red"),("无法检查 · 缺少环境依据", "amber"),("已接受 r2 / C2 / b1", "green"),("历史通过 · r1 / b1", "green")]):
        y=111+i*40
        p += [badge(736,y,label,333,tone)]
    p += [t(736,385,"读取、空集与只读", "section"),r(736,404,680,56,"wash",5),r(751,420,131,10,"skeleton",3),r(751,438,238,8,"skeleton",3),t(1027,440,"首次读取：只替换未知区", "small muted"),t(736,489,"#024 CSV 导出", "caption"),t(1055,489,"更新中 · 保留旧行", "caption blue"),control(1364,470,"更新中，保留旧内容与选中","refresh")]
    p += [r(736,512,680,58,"wash",5),t(751,545,"尚无工作", "caption muted"),button(1236,525,164,"新建任务"),r(736,584,680,58,"wash",5),t(751,617,"当前条件无结果", "caption muted"),button(1236,597,164,"清除条件"),t(736,675,"只读 · 历史仍可查阅", "caption muted"),control(1364,655,"当前监听未启用协作写操作","info")]
    p += [t(736,724,"窄屏：同一行的完整角色关系", "section"),r(736,743,390,78,"selected",4),t(752,769,"#024  CSV 导出", "caption medium"),t(752,798,"Lin → Moss · Q1 等你", "caption muted"),control(1069,759,"查看当前任务动作","more",44),t(1151,768,"44px 目标", "caption muted"),t(1151,796,"不依赖长按/hover", "caption muted")]
    p += [t(736,865,"局部信息 / 不可用原因", "section"),control(736,882,"查看状态含义","info"),r(788,872,415,74,"border",6),t(803,900,"旧依据 b1 的通过不能接受当前 b2。", "caption"),t(803,928,"查看当前结果后再请求检查。", "caption blue"),t(24,1029,"32px 桌面控件 · 44px 触摸控件 · 正文 13–14px · 元信息 12px", "small muted")]
    return [g("finite-component-state-vocabulary-not-a-product-dashboard",p)]


def write(name,title,content,height=900):
    # Extra styles belong to this design reference, never runtime component code.
    original=d.STYLE
    d.STYLE += '\n.focus-ring{fill:none;stroke:#316fd0;stroke-width:2}.invalid-field{fill:#fff;stroke:#b04c45;stroke-width:1.4}.skeleton{fill:#dfe5e9}\n'
    d.write(name,title,1440,height,content)
    d.STYLE=original


if __name__=="__main__":
    for filename,title,scene,height in [
        ("05-agent-resources.svg","Agents 资源、精确身份与关联工作",agents,900),
        ("06-create-validation.svg","本页草稿、字段校验与角色候选",create,900),
        ("07-messages-recovery.svg","角色收件人、消息回执与原操作查询",messages,900),
        ("08-check-history.svg","检查历史、具体失败与修改要求",history,900),
        ("09-result-resources.svg","跨任务结果与保存的产物引用",results,900),
        ("10-component-states.svg","实际组件、重要状态与局部反馈",sheet,1060),
        ("11-check-resources.svg","检查与接受资源、当前资格与重查理由",checks,900),
        ("12-edit-conflict.svg","已有任务依据冲突、字段对比与放弃边界",edit_conflict,900),
        ("13-narrow-picker.svg","窄屏候选选择与草稿返回",narrow_picker,844),
    ]:
        if filename=="13-narrow-picker.svg":
            d.write(filename,title,390,height,scene())
        else:
            write(filename,title,scene(),height)
