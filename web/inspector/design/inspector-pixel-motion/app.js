(() => {
  'use strict';
  const root = document.documentElement;
  const board = document.body.dataset.board;
  if (new URLSearchParams(location.search).has('embedded')) root.dataset.embedded = 'true';
  const $ = (selector, scope = document) => scope.querySelector(selector);
  const $$ = (selector, scope = document) => [...scope.querySelectorAll(selector)];
  const paths = {
    inspector:'M1 2h14v12H1V2Zm2 2v8h10V4H3Zm1 1h2v1h2v2H6v1H4V7h2V6H4V5Zm5 4h3v2H9V9Z',
    play:'M4 1h2v2h3v2h3v2h3v2h-3v2H9v2H6v2H4V1Z',
    stop:'M2 2h12v12H2V2Zm2 2v8h8V4H4Z',
    unknown:'M4 1h8v2h2v4h-2v2H9v2H7V7h5V3H4V1Zm3 11h2v3H7v-3Z',
    warning:'M7 1h2v2h2v3h2v3h2v5H1V9h2V6h2V3h2V1Zm0 5v4h2V6H7Zm0 5v2h2v-2H7Z',
    check:'M1 8h2v2h2v2h2v-2h2V8h2V6h2V4h2v3h-2v2h-2v2H9v2H7v2H5v-2H3v-2H1V8Z',
    clock:'M5 1h6v2H5V1ZM3 3h2v2H3V3ZM1 5h2v6H1V5Zm2 6h2v2H3v-2Zm2 2h6v2H5v-2Zm6-2h2v2h-2v-2Zm2-6h2v6h-2V5Zm-2-2h2v2h-2V3ZM7 4h2v4h3v2H7V4Z',
    chevron:'M3 5h2v2h2v2h2V7h2V5h2v2h-2v2H9v2H7V9H5V7H3V5Z',
    folder:'M1 3h5v2h9v9H1V3Zm2 4v5h10V7H3Z',
    layers:'M7 1h2v2h3v2h3v2h-3v2H9v2H7V9H4V7H1V5h3V3h3V1ZM1 9h2v2h4v2h2v-2h4V9h2v2h-2v2H9v2H7v-2H3v-2H1V9Z',
    arrow:'M3 3h10v10h-2V7H9V5H3V3Zm4 4h2v2H7v2H5v2H3v-2h2V9h2V7Z',
    search:'M3 1h6v2H3V1ZM1 3h2v6H1V3Zm8 0h2v6H9V3ZM3 9h6v2H3V9Zm6 2h2v2h2v2h-2v-2H9v-2Z',
    sound:'M1 5h3l4-4h2v14H8l-4-4H1V5Zm11-2h2v2h2v6h-2v2h-2v-2h2V5h-2V3Z',
    mute:'M1 5h3l4-4h2v14H8l-4-4H1V5Zm11 0h2v2h2v2h-2v2h-2V9h-2V7h2V5Z',
    sun:'M6 0h3v3H6V0Zm0 12h3v3H6v-3ZM0 6h3v3H0V6Zm12 0h3v3h-3V6ZM4 4h7v7H4V4ZM1 1h2v2H1V1Zm11 0h2v2h-2V1ZM1 12h2v2H1v-2Zm11 0h2v2h-2v-2Z',
    impact:'M7 0h2v4H7V0ZM7 12h2v4H7v-4ZM0 7h4v2H0V7Zm12 0h4v2h-4V7ZM2 2h2v2H2V2Zm10 0h2v2h-2V2ZM2 12h2v2H2v-2Zm10 0h2v2h-2v-2Z',
  };
  let iconSequence = 0;
  function icon(name, classes = '') {
    const shape = paths[name] || paths.inspector;
    const id = `mosaic-${++iconSequence}`;
    return `<svg class="icon mosaic-icon ${classes}" viewBox="0 0 16 16" aria-hidden="true"><defs><clipPath id="${id}-a"><rect x="0" y="0" width="8" height="8"/></clipPath><clipPath id="${id}-b"><rect x="8" y="4" width="8" height="6"/></clipPath><clipPath id="${id}-c"><rect x="4" y="10" width="8" height="6"/></clipPath></defs><path d="${shape}"/><g class="mosaic-tile tile-a" clip-path="url(#${id}-a)"><path d="${shape}"/></g><g class="mosaic-tile tile-b" clip-path="url(#${id}-b)"><path d="${shape}"/></g><g class="mosaic-tile tile-c" clip-path="url(#${id}-c)"><path d="${shape}"/></g></svg>`;
  }
  const badge = (label, type = 'proposal') => `<span class="badge ${type}">${label}</span>`;
  const button = (label, {id = '', tone = '', symbol = '', attrs = '', small = false} = {}) => `<button type="button" class="pixel ${tone} ${small ? 'small' : ''}" ${id ? `id="${id}" data-dc-id="${id}"` : ''} ${attrs}><span class="face">${symbol ? icon(symbol) : ''}${label}</span></button>`;
  const loader = () => '<span class="loader" aria-hidden="true"><i></i><i></i><i></i><i></i></span>';
  const panel = (id, title, content, {index = '', mark = '', classes = ''} = {}) => `<section class="panel ${classes}" data-dc-id="${id}" aria-labelledby="${id}-title"><header class="panel-header"><span class="section-index">${index}</span><h2 id="${id}-title">${title}</h2>${mark}</header>${['cursor-contract','motion-matrix'].includes(id) ? `<details class="spec-disclosure" data-dc-id="${id}-disclosure"><summary>展开完整规范与场景矩阵</summary>${content}</details>` : content}</section>`;
  const boards = [
    ['frames','阶梯框与按钮','01'], ['tools','工具光标与声音','02'], ['signals','状态与图表','03'], ['investigation','恢复调查','04'],
  ];
  const boardTitles = {
    frames:['让像素，有明确的手感','先恢复阶梯框，再看操作反馈。悬停不搬动信息，按下只下沉 2px。'],
    tools:['工具在手，动作有分寸','三把原创工具，24×24px。只在试敲区域替换指针；声音由你开启。'],
    signals:['状态有节奏，事实不跳动','运行、等待和加载各有节奏；停止与未知保持安静。'],
    investigation:['在调查现场，轻轻动一下','保留原版恢复层级与动作位置，只演示局部反馈。所有场景均为离线样例。'],
  };

  function frameDiagram() {
    return `<svg class="frame-figure" id="frame-figure" viewBox="0 0 440 208" role="img" aria-label="阶梯边框放大示意，角部仅水平竖直折进，不含斜线"><defs><pattern id="pixel-grid" width="8" height="8" patternUnits="userSpaceOnUse"><path d="M8 0H0V8" fill="none" stroke="var(--line)" stroke-width="1"/></pattern></defs><rect class="frame-grid" width="440" height="208" fill="url(#pixel-grid)"/><g shape-rendering="crispEdges"><path d="M26 28H310V44H326V158H310V174H26V158H10V44H26Z" fill="var(--shadow)" transform="translate(8 8)"/><path d="M26 28H310V44H326V158H310V174H26V158H10V44H26Z" fill="var(--edge)"/><path d="M34 36H302V52H318V150H302V166H34V150H18V52H34Z" fill="var(--blue-tint)"/><path d="M18 44H34V28M310 28V44H326" fill="none" stroke="var(--pink)" stroke-width="2"/><path d="M340 28H366M340 44H366M358 28V44" fill="none" stroke="var(--orange)" stroke-width="2"/></g><text x="374" y="40">4px 角</text><text x="54" y="93" class="diagram-label">H / V 阶梯内切</text><text x="54" y="117">轮廓与内层同构</text><text x="54" y="144">这里为 4× 放大示意</text><text x="18" y="201">缺一格，而不是斜切一刀</text><path d="M352 86H412V146H352Z" fill="var(--yellow)"/><path d="M352 86H412V146H352Z" fill="none" stroke="var(--edge)" stroke-width="2"/><text x="353" y="166">2px 网格</text></svg>`;
  }
  function timingDiagram() {
    return `<svg class="timeline" id="timing-diagram" viewBox="0 0 460 84" role="img" aria-label="按下90毫秒，下沉2像素，释放110毫秒"><path d="M14 47H446" stroke="var(--line)" stroke-width="2"/><path d="M14 28H146V32H292V28H446" fill="none" stroke="var(--blue)" stroke-width="2"/><rect class="timing-face" x="184" y="12" width="40" height="14"/><path d="M14 42V52M146 42V52M292 42V52M446 42V52" stroke="var(--muted)"/><text x="14" y="72">0</text><text x="117" y="72">按下 90ms</text><text x="269" y="72">释放 110ms</text><text x="402" y="72">结束</text></svg>`;
  }
  function framesBoard() {
    const states = [
      ['默认','idle','默认 / 可操作',''], ['悬停','hover','hover 80ms',''], ['键盘焦点','focus','focus 即时',''],
      ['按住我','pressed','press 90ms / 2px',''], ['选中','selected','selected / 持续','aria-pressed="true"'], ['禁用','disabled','disabled / 静态','disabled'],
      ['处理中','busy','busy / 防重复','disabled aria-busy="true"'], ['危险操作','danger','danger / 明确后果',''], ['次要操作','secondary','secondary / 同轮廓',''],
    ];
    const gallery = states.map(([label,state,note,attrs]) => `<div class="state-cell">${button(state === 'busy' ? `${loader()}${label}` : label, {id:`state-${state}`,tone:state === 'idle' ? 'primary' : state === 'danger' ? 'danger' : '',symbol:({idle:'play',hover:'layers',focus:'arrow',selected:'check',danger:'warning',secondary:'search'})[state] || '',attrs:`${attrs} ${['hover','focus','pressed','selected'].includes(state) ? `data-state="${state}"` : ''}`})}<span class="note">${note}</span></div>`).join('');
    return `<div class="board-grid">${panel('frame-grammar','阶梯内切 · 双层轮廓', `${frameDiagram()}<div class="frame-specs"><div><div class="spec-value">2px</div><div class="spec-key">像素网格 / 轮廓</div></div><div><div class="spec-value">2 / 4 / 6</div><div class="spec-key">标签 / 按钮 / 面板角</div></div><div><div class="spec-value">+2,+2</div><div class="spec-key">静态短硬阴影</div></div></div><div class="control-row divider">${button('网格：开',{id:'grid-toggle',small:true,attrs:'aria-pressed="true"'})}<span class="note">原轮廓已采用 · 网格归整为提案</span></div>`, {index:'01',mark:badge('语法已采用','adopted')})}${panel('button-states','基础按钮 · 九种状态', `<div class="state-grid">${gallery}</div><div class="feedback-strip" id="button-feedback" role="status">悬停图标试试：像素小块轻动一次；禁用与处理中保持静态。</div>`, {index:'02',mark:badge('动效提案')})}${panel('form-feedback','输入、筛选与开关',`<div class="control-row"><div class="input-group"><label for="sample-search">搜索 Agent / 报错</label><input class="input" id="sample-search" placeholder="输入看看焦点轮廓" autocomplete="off"></div>${button('<span class="switch-track" aria-hidden="true"></span>仅看待核实',{id:'sample-switch',attrs:'role="switch" aria-checked="false"'})}</div><div class="control-row divider">${button('全部 28',{id:'chip-all',small:true,attrs:'aria-pressed="true"'})}${button('待核实 10',{id:'chip-unknown',small:true,attrs:'aria-pressed="false"'})}${button('运行中 0',{id:'chip-disabled',small:true,attrs:'disabled'})}</div><div class="feedback-strip" id="form-feedback" role="status">输入、开关与筛选保持各自状态。</div>`,{index:'03'})}${panel('press-timing','按下与释放 · 一次完整反馈',`<div class="control-row spread"><span class="note">只动控件表面；相邻文本位置不变</span>${button('播放时序',{id:'replay-press',symbol:'play',small:true})}</div>${timingDiagram()}<div class="control-row">${button('真实按住试试',{id:'live-press',tone:'primary'})}<span class="note" id="press-readout">鼠标 / Space 均可</span></div>`,{index:'04',mark:badge('90 + 110ms')})}</div>`;
  }
  function toolImage(name, classes = '', id = '') { return `<img src="assets/${name}.svg" alt="" class="${classes}" ${id ? `id="${id}"` : ''} draggable="false">`; }
  function toolsBoard() {
    const toolCards = [['hammer','敲定锤','锤头斜握向下敲'],['shovel','探查铲','定位与查看'],['chisel','细修凿','选择与调整']].map(([name,label,hint]) => button(`${toolImage(name)}<strong>${label}</strong><span class="note">${hint}</span>`,{id:`tool-${name}`,attrs:`data-tool-choice="${name}" aria-pressed="${name === 'hammer'}"`,tone:'tool-choice'})).join('');
    const cells = Array.from({length:18}, (_,index) => `<span class="material-cell material-${index % 3}" data-cell="${index}"></span>`).join('');
    const hoverLab = [['search','搜索'],['folder','项目'],['layers','分组'],['arrow','打开']].map(([name,label]) => button('',{id:`hover-${name}`,symbol:name,small:true,attrs:`aria-label="${label}图标微动示例"`})).join('');
    const workbench = `<div class="tool-list">${toolCards}</div><div class="tool-hints"><span>斜握 → 锤头下落 → 小块飞散</span><span>文字光标保持原样</span></div><div class="workpad" id="workpad" data-tool="hammer" data-dc-id="tool-practice-pad" role="button" tabindex="0" aria-label="工具试敲台，点击方块或按空格敲落"><div class="material-patch" aria-hidden="true">${cells}</div><span class="note">点方块试敲 · 仅本地靶块</span><span class="tool-pointer" id="tool-pointer" aria-hidden="true" hidden>${toolImage('hammer','pointer-tool')}</span></div><div class="feedback-strip tool-status-row"><span id="tool-readout" role="status">敲定锤 · 0 次 · 已敲落 0 / 18</span>${button('补回方块',{id:'restore-blocks',small:true})}</div>`;
    const feedback = `<div class="tool-hero">${toolImage('hammer','tool-demo','tool-demo')}<div><h3>先抬起，锤头向下击中</h3><p class="subline">敲击 200ms · 碎屑 360ms<br>靶块真的留一个缺口，再补回</p></div></div><div class="control-row">${button('试听一次',{id:'sound-test',symbol:'sound',attrs:'disabled'})}<span class="badge warning" id="sound-state">声音关闭</span></div><div class="sound-display" id="sound-display" aria-hidden="true">${'<span class="sound-bar"></span>'.repeat(18)}</div><div class="contract-list"><div class="contract-line"><strong>默认静音</strong><span class="note">显式开启 · 短敲击 40ms</span></div><div class="contract-line hover-lab"><strong>图标微动</strong><div class="control-row">${hoverLab}</div></div><div class="contract-line"><strong>240ms · 只动 1px</strong><span class="note">悬停一次，文字与布局不动</span></div><div class="contract-line"><strong>减少动态</strong><span class="note">保留缺口，取消敲击与碎屑</span></div></div><div class="input-group divider"><label for="cursor-input">输入框光标检查</label><input id="cursor-input" class="input" placeholder="这里继续使用文字光标"></div>`;
    const contract = `<table class="matrix"><thead><tr><th>区域 / 输入</th><th>反馈</th><th>边界</th></tr></thead><tbody><tr><td>鼠标 / 试敲台</td><td>斜握工具，锤头向下，靶块缺口 + 方块碎屑</td><td>只在沙盒区域跟随，不遮挡点击</td></tr><tr><td>图标 hover / focus</td><td>三块图形错开 1px，再拼回</td><td>240ms 单次；禁用与状态事实不抖动</td></tr><tr><td>触屏 / 键盘</td><td>点击 / Enter / Space 敲落，补回可重复</td><td>不依赖悬停</td></tr><tr><td>减少动态 / 后台</td><td>即时缺口与文字，静态图标</td><td>不创建位移、碎屑或隐藏循环</td></tr></tbody></table>`;
    return `<div class="board-grid equal-grid">${panel('tool-workbench','原创工具 · 击碎试验台',workbench,{index:'01',mark:badge('方向与碎屑修订')})}${panel('tool-feedback','敲击手感与图标微动',feedback,{index:'02',mark:badge('默认静音','warning')})}${panel('cursor-contract','使用边界',contract,{index:'03',classes:'full',mark:badge('无第三方资产','adopted')})}</div>`;
  }
  function signalCard(label, note, type, symbol, color) {
    const visual = type === 'waiting' ? '<span class="signal-bars" aria-hidden="true"><i></i><i></i><i></i></span>' : type === 'loading' ? loader() : icon(symbol,'signal-icon');
    return `<div class="signal-card ${type}" data-dc-id="signal-${type}"><span class="signal-slot ${color}">${visual}</span><div><h3>${label}</h3><p>${note}</p></div></div>`;
  }
  function countChart() {
    return `<svg class="chart" id="count-chart" viewBox="0 0 520 186" role="group" aria-label="设计固定样例：488个会话，主会话68，子Agent420"><text x="8" y="22">会话构成 · 固定来源样例</text><text x="508" y="22" text-anchor="end" class="value-label">488</text><g class="data-geometry"><rect x="8" y="48" width="70" height="46" fill="var(--blue)" class="segment" tabindex="0" role="button" aria-label="主会话 68" data-chart-part="main"/><rect x="80" y="48" width="428" height="46" fill="var(--violet)" class="segment" tabindex="0" role="button" aria-label="子 Agent 420" data-chart-part="child"/></g><text x="8" y="118">主会话 68</text><text x="508" y="118" text-anchor="end">子 Agent 420</text><path d="M8 136H508" stroke="var(--line)"/><text x="8" y="162">数字始终显示最终值 · 仅裁剪显露已有几何</text></svg>`;
  }
  function signalsBoard() {
    const signals = [
      ['执行中','1200ms · 2px 跳动','running','play','blue'], ['等待回复','1400ms · 两档长度','waiting','clock','violet'],
      ['读取中','800ms · 4格扫描','loading','clock','blue'], ['已停止','静态 · 不庆祝','stopped','stop','orange'],
      ['效果未知','静态 · 不补发','unknown','unknown','red'], ['明确错误','静态 · 保留原因','error','warning','red'],
    ].map(args => signalCard(...args)).join('');
    return `<div class="board-grid equal-grid">${panel('signal-rhythms','状态节奏 ≠ 请求加载',`<div class="signal-list" id="signal-list">${signals}</div><div class="control-row divider">${button('暂停循环',{id:'pause-signals',small:true,attrs:'aria-pressed="false"'})}${button(`${icon('check','green')}明确成功事件`,{id:'confirm-success',small:true})}<span class="note" id="success-readout" role="status">成功只播放一次</span></div>`,{index:'01',mark:badge('节奏提案')})}${panel('chart-motion','图表显露 · 不生产数据',`<div class="control-row spread"><span class="note">来源：已接受 v2 · 会话 488 / 子 Agent 420</span>${button('重新显露',{id:'reveal-chart',small:true,symbol:'play'})}</div>${countChart()}<div class="feedback-strip chart-readout" id="chart-readout" role="status">悬停、聚焦或点击色段查看精确值。</div><div class="missing-chart" data-dc-id="missing-trend"><span>逐日趋势：未提供原始事实，不补零、不连线</span></div><p class="note divider">固定设计样例，不连接当前主机。主会话 68 = 488 − 420。</p>`,{index:'02',mark:badge('固定事实样例','warning')})}${panel('motion-matrix','状态与动效触发矩阵',`<table class="matrix"><thead><tr><th>触发</th><th>图形</th><th>文本 / 数值</th><th>减少动态</th></tr></thead><tbody><tr><td>运行 / 等待</td><td>固定槽内跳动 / 长度变换</td><td>持续显示真实状态名称</td><td>静态图标 + 文字</td></tr><tr><td>请求 pending</td><td>4格扫描，与运行标识分开</td><td>保留内容与旧值，不伪装空数据</td><td>静态4格 + “读取中”</td></tr><tr><td>明确成功事件</td><td>160ms 单次落定</td><td>仅事件确证，不从动画推断</td><td>立即勾选</td></tr><tr><td>停止 / 错误 / 未知</td><td>静止，无成功动画</td><td>保留原因与核实入口</td><td>完全一致</td></tr><tr><td>图表 / 选点</td><td>180ms 显露 / 80ms 换色</td><td>最终精确值，不滚动数字、不补点</td><td>完整显示最终几何</td></tr></tbody></table>`,{index:'03',classes:'full',mark:badge('事实与动效分离','adopted')})}</div>`;
  }
  const scenarios = {
    stopped:{label:'明确停止',status:'已停止 · 允许核验',icon:'stop',evidence:'已观察到明确的停止事件',delivery:'尚未请求投递',action:'预览恢复动作',canPreview:true,note:'停止标记不动；只在展开和按压时给出局部反馈。'},
    tool:{label:'仅工具错误',status:'工具错误 · 运行待核实',icon:'warning',evidence:'只有工具调用错误，未提供停止证据',delivery:'不产生恢复授权',action:'恢复不可用',canPreview:false,note:'工具失败不等于任务停止，不能由红色图标推导恢复动作。'},
    unknown:{label:'投递效果未知',status:'效果未知 · 待核实',icon:'unknown',evidence:'投递结果不能确认是否生效',delivery:'效果未知，保持屏障',action:'未知 · 不补发',canPreview:false,note:'未知状态静止；原会话核实可用，没有补发或成功反馈。'},
    accepted:{label:'宿主已接受',status:'宿主已接受',icon:'arrow',evidence:'只确认宿主接收了 follow-up',delivery:'运行结果尚未确认',action:'已接受 · 等待核实',canPreview:false,note:'接受回执不等于任务完成；不播放绿色成功动画。'},
  };
  function investigationBoard() {
    const choices = Object.entries(scenarios).map(([key,scene]) => button(scene.label,{id:`scenario-${key}`,attrs:`data-scenario="${key}" aria-pressed="${key === 'stopped'}"`,symbol:scene.icon})).join('');
    const product = `<section class="inspector-frame" data-dc-id="recovery-local-scene" aria-label="恢复调查设计样例"><nav class="product-tabs" aria-label="原结构标本"><span class="product-tab">会话</span><span class="product-tab active">恢复</span><span class="product-tab">看板</span><span class="product-tab">Token</span><span class="product-tab">明细</span><span class="product-tab">横评</span><span class="product-tab">事件流</span><span class="product-tab">智能分析</span></nav><div class="recovery-toolbar"><div class="control-row spread"><span class="note">自动恢复策略</span>${badge('自动关闭','warning')}</div><div class="control-row">${button('全部 1',{id:'scene-filter-all',small:true,attrs:'aria-pressed="true"'})}${badge('5m → 20m → 1h','proposal')}<span class="note">已有策略标本</span></div><div class="input-group"><label for="scene-search">搜索 Agent / 报错</label><input id="scene-search" class="input" placeholder="搜索此设计样例" autocomplete="off"></div></div><div class="project-heading">${icon('chevron')}${icon('folder','blue')}Agent Inspector <span class="muted">1</span></div><div class="workspace-heading">${icon('layers','violet')}Design and implement Paseo bulk agent recovery plugin</div><article class="agent-record" id="sample-agent"><div class="agent-summary"><div class="agent-identity"><p class="agent-title">Agent Inspector · 工作台体验领域负责人</p><p class="agent-meta">codex · 设计样例 · 非当前运行状态</p></div><span class="record-status" id="record-status">${icon('stop')}已停止 · 允许核验</span>${button('收起',{id:'details-toggle',small:true,attrs:'aria-expanded="true" aria-controls="agent-details"'})}</div><p class="error-excerpt">${icon('warning')}We’re currently experiencing high demand, which may cause temporary errors.</p><div class="agent-details" id="agent-details"><dl class="detail-facts"><dt>状态证据</dt><dd id="scene-evidence">已观察到明确的停止事件</dd><dt>投递状态</dt><dd id="scene-delivery">尚未请求投递</dd><dt>原始错误</dt><dd>上游高负载摘录；它本身不是停止证据</dd></dl><div class="control-row">${button('预览恢复动作',{id:'preview-recovery',tone:'primary',symbol:'play'})}${button('打开原会话',{id:'original-agent',symbol:'arrow',small:true})}</div><p class="note divider">这里没有真实发送通道；动作只展示局部视觉结果。</p></div></article><div class="feedback-strip" id="scene-feedback" role="status">当前为“明确停止”设计场景，未发送任何指令。</div><footer class="product-footer"><span>本地画板 · 固定样例</span><span>未知不补发</span></footer></section>`;
    const sidebar = panel('scenario-switcher','切换事实场景',`<div class="scenario-choice" aria-label="设计场景">${choices}</div><div class="scenario-info" data-dc-id="scenario-rule"><strong id="scenario-title">明确停止</strong><p class="note" id="scenario-note">停止标记不动；只在展开和按压时给出局部反馈。</p></div><div class="contract-list"><div class="contract-line"><strong>保留原层级</strong><span class="note">Project → Workspace → Agent</span></div><div class="contract-line"><strong>就地展开</strong><span class="note">140ms，草稿与焦点保留</span></div><div class="contract-line"><strong>自动保持关闭</strong><span class="note">设计不授予自动权限</span></div></div>`,{index:'02',mark:badge('本地模拟','warning')});
    return `<div class="investigation-grid">${product}${sidebar}</div><dialog id="preview-dialog" class="dialog" aria-labelledby="dialog-title"><h2 id="dialog-title">恢复动作 · 视觉预览</h2><p class="muted">展示按钮按压和反馈位置。此画板不发送 follow-up，不产生“恢复成功”结果。</p><div class="control-row">${button('看过，关闭',{id:'close-dialog',tone:'primary'})}</div></dialog>`;
  }
  function shell(content) {
    const [title, subtitle] = boardTitles[board];
    return `<div class="shell"><header class="topbar"><span class="brandmark">${icon('inspector')}</span><div><div class="brand-title">Agent Inspector</div><div class="topbar-title">像素动效实验台 / 本地设计</div></div><div class="preferences" aria-label="演示偏好">${button('动态：系统',{id:'motion-toggle',small:true,attrs:'aria-pressed="false"'})}${button('声音：关',{id:'sound-toggle',small:true,symbol:'mute',attrs:'aria-pressed="false"'})}${button('切换深色',{id:'theme-toggle',small:true,symbol:'sun'})}</div></header><nav class="navbar" aria-label="动效画板">${boards.map(([id,label,index]) => `<a class="navlink" href="${id}.html" ${id === board ? 'aria-current="page"' : ''}><span class="mono">${index}</span>${label}</a>`).join('')}<span class="nav-label">2px GRID · LOCAL ONLY</span></nav><main class="main"><div class="page-intro"><div><p class="eyebrow">MOTION STUDY / ${boards.find(item => item[0] === board)[2]}</p><h1>${title}</h1><p class="subline">${subtitle}</p></div><div class="intro-marks">${badge('鲜彩 v2 已采用','adopted')}${badge('本轮动效提案')}</div></div>${content}<footer class="footer"><span>鲜彩 v2 · 阶梯内切角 · 原创工具 · 默认静音</span><span id="global-feedback" role="status">离线设计 · 不连接业务数据或动作</span></footer></main></div>`;
  }
  const renderers = {frames:framesBoard,tools:toolsBoard,signals:signalsBoard,investigation:investigationBoard};
  $('#app').innerHTML = shell(renderers[board]());

  const systemMotion = matchMedia('(prefers-reduced-motion: reduce)');
  let userReduced = false;
  let soundEnabled = false;
  let audioContext;
  let soundAttempt = 0;
  let lastSound = -Infinity;
  let currentTool = 'hammer';
  let tapCount = 0;
  const toolNames = {hammer:'敲定锤',shovel:'探查铲',chisel:'细修凿'};
  const reduced = () => systemMotion.matches || userReduced;
  function report(text) { $('#global-feedback').textContent = text; }
  function applyMotion() {
    root.dataset.reduced = String(reduced());
    $('#motion-toggle .face').textContent = reduced() ? '动态：减少' : '动态：系统';
    $('#motion-toggle').setAttribute('aria-pressed', String(reduced()));
    $$('.impact').forEach(item => item.remove());
  }
  function replay(element, className) {
    element.classList.remove(className);
    if (reduced()) return;
    void element.offsetWidth;
    element.classList.add(className);
  }
  function clickSound() {
    if (!soundEnabled || !audioContext || audioContext.state !== 'running' || document.hidden) return;
    const now = performance.now();
    if (now - lastSound < 100) return;
    lastSound = now;
    const start = audioContext.currentTime;
    const oscillator = audioContext.createOscillator();
    const gain = audioContext.createGain();
    oscillator.type = 'triangle';
    oscillator.frequency.setValueAtTime(currentTool === 'chisel' ? 760 : currentTool === 'shovel' ? 260 : 440,start);
    oscillator.frequency.exponentialRampToValueAtTime(100,start + .04);
    gain.gain.setValueAtTime(.055,start);
    gain.gain.exponentialRampToValueAtTime(.001,start + .04);
    oscillator.connect(gain).connect(audioContext.destination);
    oscillator.start(start);
    oscillator.stop(start + .04);
    oscillator.onended = () => { oscillator.disconnect(); gain.disconnect(); };
    const display = $('#sound-display');
    if (display) replay(display,'audible');
  }
  function refreshSound() {
    $('#sound-toggle').setAttribute('aria-pressed', String(soundEnabled));
    $('#sound-toggle .face').innerHTML = `${icon(soundEnabled ? 'sound' : 'mute')}声音：${soundEnabled ? '开' : '关'}`;
    if ($('#sound-test')) $('#sound-test').disabled = !soundEnabled;
    if ($('#sound-state')) $('#sound-state').textContent = soundEnabled ? '已显式开启' : '声音关闭';
  }
  async function setSound() {
    const attempt = ++soundAttempt;
    if (soundEnabled) {
      soundEnabled = false;
      refreshSound();
      if (audioContext) { const previous = audioContext; audioContext = undefined; await previous.close(); }
      report('声音已关闭');
      return;
    }
    const Audio = window.AudioContext || window.webkitAudioContext;
    if (!Audio) { report('此浏览器没有可用的声音能力，保持静音'); return; }
    try {
      const next = new Audio();
      await next.resume();
      if (attempt !== soundAttempt || document.hidden) { await next.close(); return; }
      audioContext = next;
      soundEnabled = next.state === 'running';
      refreshSound();
      report(soundEnabled ? '声音已显式开启 · 点击为本地短敲击' : '声音未就绪，保持静音');
    } catch { soundEnabled = false; refreshSound(); report('声音未能启用，保持静音'); }
  }
  $('#motion-toggle').addEventListener('click', () => { userReduced = !userReduced; applyMotion(); report(reduced() ? '已减少动态，所有事实和控件保持可用' : '恢复系统动态偏好'); });
  systemMotion.addEventListener('change', applyMotion);
  $('#sound-toggle').addEventListener('click', setSound);
  $('#theme-toggle').addEventListener('click', () => {
    root.dataset.theme = root.dataset.theme === 'dark' ? 'light' : 'dark';
    $('#theme-toggle .face').innerHTML = `${icon('sun')}切换${root.dataset.theme === 'dark' ? '明亮' : '深色'}`;
  });
  document.addEventListener('keydown', event => {
    const control = event.target.closest('.pixel');
    if (control && !control.disabled && event.code === 'Space') control.dataset.held = 'true';
  });
  document.addEventListener('keyup', event => { if (event.code === 'Space') $$('.pixel[data-held]').forEach(control => delete control.dataset.held); });
  window.addEventListener('blur', () => $$('.pixel[data-held]').forEach(control => delete control.dataset.held));
  document.addEventListener('click', event => {
    const control = event.target.closest('button');
    if (control && !control.disabled && control.id !== 'sound-toggle' && control.id !== 'sound-test') clickSound();
  });
  document.addEventListener('visibilitychange', () => {
    root.dataset.hidden = String(document.hidden);
    if (!document.hidden) return;
    ++soundAttempt;
    soundEnabled = false;
    refreshSound();
    if (audioContext) { const previous = audioContext; audioContext = undefined; previous.close(); }
    $$('.impact').forEach(item => item.remove());
  });
  window.addEventListener('pagehide', () => { ++soundAttempt; if (audioContext) audioContext.close(); });
  applyMotion();
  document.addEventListener('inspector-preview-preferences', event => {
    const preference = event.detail;
    if (preference.theme === 'light' || preference.theme === 'dark') root.dataset.theme = preference.theme;
    userReduced = preference.reduced === true;
    applyMotion();
    if (typeof preference.sound === 'boolean' && preference.sound !== soundEnabled) setSound();
  });

  function bindFrames() {
    $('#grid-toggle').addEventListener('click', () => {
      const hidden = $('#frame-figure').classList.toggle('no-grid');
      $('#grid-toggle').setAttribute('aria-pressed', String(!hidden));
      $('#grid-toggle .face').textContent = `网格：${hidden ? '关' : '开'}`;
    });
    $$('.state-grid button:not(:disabled)').forEach(control => control.addEventListener('click', () => {
      if (control.id === 'state-selected') { delete control.dataset.state; control.setAttribute('aria-pressed', String(control.getAttribute('aria-pressed') !== 'true')); }
      $('#button-feedback').textContent = `本地操作已接收：${control.textContent.trim()}。这不是业务成功回执。`;
    }));
    $('#sample-search').addEventListener('input', event => { $('#form-feedback').textContent = `搜索草稿：${event.target.value || '空'} · 未发送查询`; });
    $('#sample-switch').addEventListener('click', () => {
      const control = $('#sample-switch');
      control.setAttribute('aria-checked', String(control.getAttribute('aria-checked') !== 'true'));
      $('#form-feedback').textContent = `仅看待核实：${control.getAttribute('aria-checked') === 'true' ? '开' : '关'} · 输入草稿保留`;
    });
    [$('#chip-all'),$('#chip-unknown')].forEach(control => control.addEventListener('click', () => {
      [$('#chip-all'),$('#chip-unknown')].forEach(item => item.setAttribute('aria-pressed',String(item === control)));
      $('#form-feedback').textContent = `已选择：${control.textContent.trim()} · 样例筛选`;
    }));
    $('#replay-press').addEventListener('click', () => { replay($('#timing-diagram'),'is-playing'); $('#press-readout').textContent = reduced() ? '减少动态：以颜色反馈' : '90ms 下沉 → 110ms 释放'; });
    $('#live-press').addEventListener('click', () => { $('#press-readout').textContent = '本地按压已接收，相邻信息没有移动'; });
    replay($('#timing-diagram'),'is-playing');
  }
  function bindTools() {
    const pad = $('#workpad');
    const follower = $('#tool-pointer');
    const finePointer = matchMedia('(hover: hover) and (pointer: fine)');
    const updateReadout = () => { $('#tool-readout').textContent = `${toolNames[currentTool]} · ${tapCount} 次 · 已敲落 ${$$('.material-cell.broken').length} / 18`; };
    $$('[data-tool-choice]').forEach(control => control.addEventListener('click', () => {
      currentTool = control.dataset.toolChoice;
      $$('[data-tool-choice]').forEach(item => item.setAttribute('aria-pressed',String(item === control)));
      pad.dataset.tool = currentTool;
      $('#tool-demo').src = `assets/${currentTool}.svg`;
      $('.pointer-tool').src = `assets/${currentTool}.svg`;
      updateReadout();
    }));
    pad.addEventListener('pointermove', event => {
      if (!finePointer.matches || event.pointerType !== 'mouse' || reduced()) return;
      const bounds = pad.getBoundingClientRect();
      follower.hidden = false;
      pad.dataset.following = 'true';
      follower.style.left = `${event.clientX - bounds.left}px`;
      follower.style.top = `${event.clientY - bounds.top}px`;
    });
    pad.addEventListener('pointerleave', () => { follower.hidden = true; delete pad.dataset.following; });
    function strike(x, y, target) {
      tapCount++;
      const available = $$('.material-cell:not(.broken)');
      const bounds = pad.getBoundingClientRect();
      let cell = target?.closest('.material-cell:not(.broken)');
      if (!cell && available.length) cell = available.reduce((best, next) => {
        const distance = item => { const r = item.getBoundingClientRect(); return Math.hypot(r.x + r.width / 2 - bounds.x - x,r.y + r.height / 2 - bounds.y - y); };
        return distance(next) < distance(best) ? next : best;
      });
      if (cell) {
        const r = cell.getBoundingClientRect();
        x = r.x + r.width / 2 - bounds.x;
        y = r.y + r.height / 2 - bounds.y;
        cell.classList.add('broken');
      }
      updateReadout();
      replay($('#tool-demo'),'striking');
      replay($('.pointer-tool'),'striking');
      if (!reduced()) {
        const impact = document.createElement('span');
        impact.className = 'impact block-impact';
        impact.setAttribute('aria-hidden','true');
        impact.style.left = `${x}px`;
        impact.style.top = `${y}px`;
        impact.innerHTML = `<img class="strike-tool" src="assets/${currentTool}.svg" alt="">${'<i class="debris"></i>'.repeat(5)}<i class="contact-square"></i>`;
        impact.addEventListener('animationend', event => { if (event.target === impact) impact.remove(); });
        pad.append(impact);
        while ($$('.impact').length > 8) $('.impact').remove();
      }
      clickSound();
    }
    pad.addEventListener('click', event => {
      const bounds = pad.getBoundingClientRect();
      strike(event.detail === 0 ? bounds.width / 2 : event.clientX - bounds.left,event.detail === 0 ? bounds.height / 2 : event.clientY - bounds.top,event.target);
    });
    pad.addEventListener('keydown', event => {
      if (event.code !== 'Space' && event.code !== 'Enter') return;
      event.preventDefault();
      if (event.repeat) return;
      const bounds = pad.getBoundingClientRect();
      strike(bounds.width / 2,bounds.height / 2);
    });
    $('#restore-blocks').addEventListener('click', () => { $$('.material-cell.broken').forEach(cell => cell.classList.remove('broken')); $$('.impact').forEach(item => item.remove()); updateReadout(); });
    $('#sound-test').addEventListener('click', () => { clickSound(); report('已试听一次本地短敲击'); });
    $$('.hover-lab button').forEach(control => control.addEventListener('click', () => report('图标微动样例 · 单次，不移动信息布局')));
  }
  function bindSignals() {
    $('#pause-signals').addEventListener('click', () => {
      const paused = $('#signal-list').classList.toggle('loops-paused');
      $('#pause-signals').setAttribute('aria-pressed',String(paused));
      $('#pause-signals .face').textContent = paused ? '继续循环' : '暂停循环';
    });
    $('#confirm-success').addEventListener('click', () => {
      replay($('#confirm-success .icon'),'success-pop');
      $('#success-readout').textContent = '已确认成功 · 单次落定';
    });
    $('#reveal-chart').addEventListener('click', () => { replay($('#count-chart'),'is-revealing'); });
    function selectPart(control) {
      const main = control.dataset.chartPart === 'main';
      $('#chart-readout').textContent = main ? '主会话 68 = 总量 488 − 子 Agent 420（固定设计样例）' : '子 Agent 420 / 总量 488（固定设计样例）';
    }
    $$('.segment').forEach(control => {
      ['pointerenter','focus','click'].forEach(event => control.addEventListener(event,() => selectPart(control)));
      control.addEventListener('keydown', event => { if (event.code === 'Space' || event.code === 'Enter') { event.preventDefault(); selectPart(control); } });
    });
    replay($('#count-chart'),'is-revealing');
  }
  function bindInvestigation() {
    let currentScenario = 'stopped';
    const preview = $('#preview-recovery');
    $$('[data-scenario]').forEach(control => control.addEventListener('click', () => {
      currentScenario = control.dataset.scenario;
      const scene = scenarios[currentScenario];
      $$('[data-scenario]').forEach(item => item.setAttribute('aria-pressed',String(item === control)));
      $('#record-status').innerHTML = `${icon(scene.icon)}${scene.status}`;
      $('#scene-evidence').textContent = scene.evidence;
      $('#scene-delivery').textContent = scene.delivery;
      $('#scenario-title').textContent = scene.label;
      $('#scenario-note').textContent = scene.note;
      preview.disabled = !scene.canPreview;
      preview.innerHTML = `<span class="face">${icon(scene.canPreview ? 'play' : scene.icon)}${scene.action}</span>`;
      $('#scene-feedback').textContent = `当前为“${scene.label}”设计场景，未发送任何指令。`;
    }));
    $('#details-toggle').addEventListener('click', () => {
      const details = $('#agent-details');
      details.hidden = !details.hidden;
      $('#details-toggle').setAttribute('aria-expanded',String(!details.hidden));
      $('#details-toggle .face').textContent = details.hidden ? '展开' : '收起';
    });
    $('#scene-search').addEventListener('input', event => {
      const query = event.target.value.trim().toLowerCase();
      const matches = $('#sample-agent').textContent.toLowerCase().includes(query);
      $('#sample-agent').hidden = !matches;
      $('#scene-feedback').textContent = matches ? `“${scenarios[currentScenario].label}”场景 · 本地匹配 1 项` : '本地匹配 0 项 · 搜索草稿保留';
    });
    $('#scene-filter-all').addEventListener('click', () => { $('#scene-search').value = ''; $('#scene-search').dispatchEvent(new Event('input')); });
    preview.addEventListener('click', () => { if (scenarios[currentScenario].canPreview) $('#preview-dialog').showModal(); });
    $('#close-dialog').addEventListener('click', () => $('#preview-dialog').close());
    $('#original-agent').addEventListener('click', () => { $('#scene-feedback').textContent = '原会话入口演示：产品中转向原 Agent；此画板保持离线。'; });
  }
  ({frames:bindFrames,tools:bindTools,signals:bindSignals,investigation:bindInvestigation})[board]();
})();
