const protocol = 'agentx.context.v1';
// Ordinary LAN HTTP has getRandomValues, but need not expose randomUUID.
const nextId = () => `demo-${Date.now()}-${Array.from(crypto.getRandomValues(new Uint32Array(4)), part => part.toString(16)).join('-')}`;

const exactKey = target => JSON.stringify([target?.scope, target?.generation, target?.agent?.name, target?.agent?.incarnation, target?.agent?.stopped_event]);

function selectedBinding(value) {
  const target = value.target;
  if (value.status !== 'selected' || typeof value.selectionRevision !== 'string' || !value.selectionRevision || typeof target?.scope !== 'string' || typeof target.generation !== 'string' || typeof target.agent?.name !== 'string' || typeof target.agent.incarnation !== 'string' || target.agent.stopped_event !== null) throw new Error(value.error || 'Space 未返回可用的精确当前对话');
  return { target, selectionRevision: value.selectionRevision, title: value.title };
}

/** One complete Space document; capture bindings and add results never choose a member. */
export class ContextBridge {
  constructor(frame, origin, onState) {
    this.frame = frame;
    this.origin = origin;
    this.onState = onState;
    this.ready = false;
    this.started = false;
    this.pending = new Map();
    this.receive = this.receive.bind(this);
    this.loaded = this.loaded.bind(this);
    window.addEventListener('message', this.receive);
    frame.addEventListener('load', this.loaded);
  }

  connect() {
    this.started = true;
    this.reset('Space 已重新载入，请核对原草稿；捕获内容不会自动重投');
    this.onState('loading', '正在打开完整 Space…');
    this.frame.src = `${this.origin}/space/`;
    this.armTimeout();
  }

  armTimeout() {
    clearTimeout(this.readyTimer);
    this.readyTimer = setTimeout(() => {
      if (!this.ready) { clearInterval(this.helloTimer); this.onState('error', 'Space 连接超时，可在连接信息中重新载入'); }
    }, 20000);
  }

  loaded() {
    if (!this.started) return;
    this.reset('Space 已重新载入，请核对原草稿；捕获内容不会自动重投');
    this.onState('loading', '正在连接 Space 上下文功能…');
    this.helloId = nextId();
    const hello = () => this.frame.contentWindow?.postMessage({ protocol, type: 'hello', requestId: this.helloId }, this.origin);
    hello();
    this.helloTimer = setInterval(hello, 500);
    this.armTimeout();
  }

  receive(event) {
    if (event.source !== this.frame.contentWindow || event.origin !== this.origin || event.data?.protocol !== protocol) return;
    const data = event.data;
    if (data.type === 'ready' && data.requestId === this.helloId) {
      this.ready = true;
      clearInterval(this.helloTimer);
      clearTimeout(this.readyTimer);
      this.onState('ready', 'Space 已连接 · 成员与附件在 Space 中操作');
      return;
    }
    const request = this.pending.get(data.requestId);
    if (!request) return;
    if (data.type !== request.expected && !(data.type === 'result' && data.status === 'error')) return;
    clearTimeout(request.timer);
    this.pending.delete(data.requestId);
    if (data.status === 'error' || data.status === 'unavailable') request.reject(new Error(data.error || '当前对话不可用，请在 Space 中选择可发送成员'));
    else request.resolve(data);
  }

  request(type, expected, fields, timeout) {
    if (!this.ready) return Promise.reject(new Error('请等待 Space 连接后重试'));
    if (this.pending.size) return Promise.reject(new Error('上一份上下文仍在处理中'));
    const requestId = nextId();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(requestId);
        reject(new Error(type === 'addContext' ? '结果未确认，请先检查原对话草稿，避免重复添加' : '当前对话读取超时，请稍后明确重试'));
      }, timeout);
      this.pending.set(requestId, { resolve, reject, timer, expected });
      this.frame.contentWindow.postMessage({ protocol, type, requestId, ...fields }, this.origin);
    });
  }

  async captureTarget() {
    return selectedBinding(await this.request('getTarget', 'target', {}, 10000));
  }

  async add(context, binding) {
    const result = await this.request('addContext', 'result', { target: binding.target, selectionRevision: binding.selectionRevision, context }, 45000);
    if (result.status !== 'added' || exactKey(result.target) !== exactKey(binding.target) || result.selectionRevision !== binding.selectionRevision) throw new Error('添加结果身份未确认，请检查原对话草稿');
    return result;
  }

  reset(message) {
    this.ready = false;
    this.helloId = null;
    clearInterval(this.helloTimer);
    clearTimeout(this.readyTimer);
    for (const request of this.pending.values()) { clearTimeout(request.timer); request.reject(new Error(message)); }
    this.pending.clear();
  }

  dispose() {
    this.reset('页面已关闭');
    window.removeEventListener('message', this.receive);
    this.frame.removeEventListener('load', this.loaded);
  }
}
