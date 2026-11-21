import { ContextBridge } from './bridge.js';
import { orders, orderContext, renderOrderQueue, renderOrderDetail } from './scenario.js';

const $ = selector => document.querySelector(selector);
const state = { order: orders[0], bridge: null, captured: null, selection: '', busy: false, bridgeState: 'idle', binding: null, returnFocus: null, connected: false };
const assistant = $('#assistant');
const captureButtons = ['#ask-summary', '#ask-reply', '#ask-policy', '#ask-selection'];

function report(text) { $('#bridge-status').textContent = text; }
function updateControls() {
  for (const selector of captureButtons) $(selector).disabled = state.busy;
  $('#add-context').disabled = state.busy || !state.captured || state.bridgeState !== 'ready' || !state.binding;
  $('#reconnect').disabled = state.busy || !state.bridge;
  $('#clear-capture').disabled = state.busy;
  $('#rebind-context').disabled = state.busy || !state.captured || state.bridgeState !== 'ready';
  $('#capture-summary').hidden = !state.captured;
  $('#capture-title').textContent = state.captured ? `${state.captured.title}${state.binding ? ` → ${state.binding.target.agent.name}` : ' · 未关联对话'}` : '';
  $('#capture-title').title = $('#capture-title').textContent;
}

function openAssistant(invoker = document.activeElement) {
  if (assistant.hidden) {
    state.returnFocus = invoker;
    assistant.hidden = false;
    $('#launcher').setAttribute('aria-expanded', 'true');
    $('#close-assistant').focus();
  }
  if (state.bridge && !state.connected) { state.connected = true; state.bridge.connect(); }
}
function closeAssistant() {
  assistant.hidden = true;
  $('#launcher').setAttribute('aria-expanded', 'false');
  const destination = state.returnFocus?.isConnected && !state.returnFocus.disabled ? state.returnFocus : $('#launcher');
  destination.focus();
}

function selectOrder(order) {
  state.order = order;
  renderOrderQueue($('#order-list'), order.id, selectOrder);
  renderOrderDetail(order);
}

async function captureContext(kind, invoker) {
  let text;
  if (kind === 'selection') {
    if (!state.selection.trim()) { openAssistant(invoker); report('请先在订单页面选中文字，再点击“选中文字”'); return; }
    text = `请结合这段订单页面选文协助分析：\n${state.selection.trim()}`;
  } else {
    const question = kind === 'reply' ? '请根据已知事实拟一份简洁、体贴的客户回复，不承诺尚未核实的处理结果。' : kind === 'policy' ? '请结合适用规则，列出需要补充的信息和处理步骤。' : state.order.question;
    text = `${orderContext(state.order)}\n\n${question}`;
  }
  state.binding = null;
  state.captured = { title: `${state.order.id} · ${state.order.issue}`, text, url: location.href };
  openAssistant(invoker);
  updateControls();
  report('页面信息已保留 · 正在读取 Space 当前选择…');
  if (state.bridgeState === 'ready') await bindCapture();
  else report('页面信息已保留 · 在 Space 选择成员后，点击“关联当前对话”');
}

async function bindCapture() {
  if (!state.captured || state.busy || state.bridgeState !== 'ready') return;
  state.busy = true;
  updateControls();
  try {
    state.binding = await state.bridge.captureTarget();
    report(`已关联 ${state.binding.target.agent.name} · 点击“加入草稿”；切换成员后请重新关联`);
  } catch (error) { state.binding = null; report(`内容已保留：${error.message}；选择成员后可再次关联`); }
  finally { state.busy = false; updateControls(); }
}

async function addCaptured() {
  if (!state.captured || !state.binding || state.busy || state.bridgeState !== 'ready') return;
  const binding = state.binding;
  const context = state.captured;
  state.busy = true;
  updateControls();
  report('正在加入 Space 当前选择的精确对话…');
  try {
    const result = await state.bridge.add(context, binding);
    state.captured = null;
    state.binding = null;
    report(`已加入 ${result.target.agent.name} 的草稿 · 请在 Space 中审阅后发送`);
  } catch (error) { state.binding = null; report(`未确认添加：${error.message}；内容仍保留，请先检查原草稿，再明确关联`); }
  finally { state.busy = false; updateControls(); }
}

function bindInteractions() {
  $('#launcher').onclick = () => assistant.hidden ? openAssistant($('#launcher')) : closeAssistant();
  $('#close-assistant').onclick = closeAssistant;
  document.addEventListener('keydown', event => { if (event.key === 'Escape' && !assistant.hidden) { event.preventDefault(); closeAssistant(); } });
  $('#settings-toggle').onclick = () => { const panel = $('#connection-details'); panel.hidden = !panel.hidden; $('#settings-toggle').setAttribute('aria-expanded', String(!panel.hidden)); };
  $('#reconnect').onclick = () => { state.bridge.connect(); report('正在重新载入 Space；父页捕获内容保留，子页草稿可能丢失'); };
  for (const [selector, kind] of [['#ask-summary', 'summary'], ['#ask-reply', 'reply'], ['#ask-policy', 'policy'], ['#ask-selection', 'selection']]) $(selector).onclick = event => captureContext(kind, event.currentTarget);
  $('#add-context').onclick = addCaptured;
  $('#rebind-context').onclick = bindCapture;
  $('#clear-capture').onclick = () => { state.captured = null; state.binding = null; updateControls(); report('已清除父页捕获内容'); };
  document.addEventListener('selectionchange', () => {
    const selection = window.getSelection();
    if (selection?.toString().trim() && $('#workspace').contains(selection.anchorNode) && $('#workspace').contains(selection.focusNode)) state.selection = selection.toString().slice(0, 12000);
  });
}

async function start() {
  selectOrder(state.order);
  bindInteractions();
  updateControls();
  try {
    const response = await fetch('/api/config', { signal: AbortSignal.timeout(15000) });
    const config = await response.json();
    if (!response.ok) throw new Error(config.error || `配置读取失败 (${response.status})`);
    $('#service-origin').value = config.origin;
    state.bridge = new ContextBridge($('#agent-frame'), config.origin, (phase, text) => {
      state.bridgeState = phase;
      $('#agent-frame').hidden = phase === 'idle';
      $('#frame-empty').hidden = phase !== 'idle';
      report(text);
      updateControls();
    });
    if (config.extension) {
      const download = $('#download-extension');
      download.href = '/downloads/agentx-browser-context.zip';
      download.download = config.extension.filename;
      download.removeAttribute('aria-disabled');
      download.textContent = '↓ Chrome / Edge 扩展';
      download.title = `${config.extension.version || '扩展 ZIP'} · 解压后在扩展管理页加载已解压的扩展${config.extension.sha256 ? ` · SHA-256 ${config.extension.sha256}` : ''}`;
    }
    if (!assistant.hidden) { state.connected = true; state.bridge.connect(); }
  } catch (error) { report(`初始化失败：${error.message}；请刷新页面`); }
}

void start();
