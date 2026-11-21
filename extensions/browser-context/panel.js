/** Host chrome only; full Space owns all office/member/conversation/attachment state. */
import {serviceOrigin} from './service.js';
import {snapshot,capture} from './capture.js';
import {ContextBridge} from './bridge.js';
const byId = id => document.getElementById(id);
const frame = byId('conversation');
let origin = null, epoch = 0, connectEpoch = 0, busy = false, permitted = false, retainedURL = null;
function status(message,error = false) {
  byId('status').textContent = message; byId('status').parentElement.dataset.error = String(error); updateControls();
}
const bridge = new ContextBridge(frame,status);
frame.addEventListener('load',() => { epoch++; });
function updateControls() {
  for (const button of document.querySelectorAll('[data-capture]')) button.disabled = busy || !permitted || !bridge.ready || !!bridge.pending;
  byId('retry').hidden = !bridge.pending || !!bridge.pending.resolve || bridge.pending.invalidated;
  byId('retry').disabled = busy || !bridge.ready;
}
function clearRetained() {
  if (retainedURL) URL.revokeObjectURL(retainedURL);
  retainedURL = null; byId('retained').hidden = true; byId('capture-copy').value = ''; byId('capture-image').removeAttribute('src'); byId('capture-download').removeAttribute('href');
}
function retain(context) {
  clearRetained(); byId('retained').hidden = false;
  byId('capture-copy').value = [context.title,context.text,context.url].filter(Boolean).join('\n\n');
  byId('capture-copy').hidden = !byId('capture-copy').value;
  const image = context.files?.[0];
  byId('capture-image').hidden = !image; byId('capture-download').hidden = !image;
  if (image) {
    retainedURL = URL.createObjectURL(image); byId('capture-image').src = retainedURL;
    byId('capture-download').href = retainedURL; byId('capture-download').download = image.name;
  }
}
async function connect() {
  const token = ++connectEpoch;
  try {
    const nextOrigin = serviceOrigin(byId('service').value.trim());
    // The parent performs no AgentX HTTP fetch. The full Space is framed by frame-src; only page capture uses activeTab.
    permitted = true; updateControls();
    if (nextOrigin !== origin) {
      epoch++; origin = nextOrigin; bridge.reset(origin); frame.removeAttribute('src'); frame.hidden = true; byId('empty').hidden = false;
    }
    if (token !== connectEpoch) return;
    permitted = true; await chrome.storage.local.set({service:origin});
    if (token !== connectEpoch) return;
    byId('settings').open = false;
    if (frame.getAttribute('src') && bridge.ready && !bridge.pending?.invalidated) { status('Space 保持打开，成员与草稿不变'); return; }
    epoch++; bridge.reset(origin); frame.src = origin + '/space/'; frame.hidden = false; byId('empty').hidden = true;
    status('正在打开完整 Space…');
  } catch(error) {
    if (token !== connectEpoch) return;
    permitted = false; byId('settings').open = true; status(error.message,true);
  }
}
async function runCapture(kind) {
  let context;
  const boundEpoch = epoch;
  try {
    if (!permitted || !bridge.ready || bridge.pending || busy) throw new Error('请先连接 Space 或处理上一请求');
    busy = true; status('正在确认 Space 当前成员…');
    const selection = await bridge.selection();
    const tab = await snapshot();
    status('正在采集网页…'); context = await capture(kind,tab);
    if (boundEpoch !== epoch) throw new Error('采集期间服务或 Space 文档已变化；本次内容未转交新服务');
    status(`正在加入 ${selection.target.agent.name} 草稿…`);
    await bridge.send(selection,context); clearRetained();
  } catch(error) { if (context) retain(context); status(error.message,true); }
  finally { busy = false; updateControls(); }
}
byId('connection').addEventListener('submit',event => { event.preventDefault(); void connect(); });
for (const kind of ['selection','page','screenshot']) byId(kind).addEventListener('click',() => void runCapture(kind));
byId('retry').addEventListener('click',async () => {
  try { busy = true; status('正在核对原请求…'); await bridge.dispatch(); clearRetained(); }
  catch(error) { status(error.message,true); }
  finally { busy = false; updateControls(); }
});
byId('discard-copy').addEventListener('click',clearRetained);
window.addEventListener('pagehide',() => { if (retainedURL) URL.revokeObjectURL(retainedURL); });
updateControls();
chrome.storage.local.get('service').then(saved => { if (saved.service) byId('service').value = saved.service; return connect(); }).catch(error => status(error.message,true));
