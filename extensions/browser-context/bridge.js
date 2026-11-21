/** Current-selection bridge. Wire and commit guards: canonical Space live/context.ts. */
const protocol = 'agentx.context.v1';
const sameTarget = (a,b) => a?.scope === b?.scope && a?.generation === b?.generation && a?.agent?.name === b?.agent?.name && a?.agent?.incarnation === b?.agent?.incarnation && a?.agent?.stopped_event === b?.agent?.stopped_event;
export class ContextBridge {
  constructor(frame, onState) {
    this.frame = frame; this.onState = onState; this.ready = false; this.pending = null; this.query = null;
    window.addEventListener('message', event => this.receive(event));
    frame.addEventListener('load', () => this.loaded());
    this.timer = setInterval(() => { if (this.origin && !this.ready && frame.getAttribute('src')) this.hello(); }, 1000);
  }
  loaded() {
    this.ready = false;
    if (this.query) { clearTimeout(this.query.timer); this.query.reject(new Error('Space 已重新加载，请重新采集')); this.query = null; }
    if (this.pending) {
      clearTimeout(this.pending.timer); this.pending.invalidated = true;
      this.pending.reject?.(new Error('Space 已重载；去重缓存可能丢失，请核对原会话草稿，勿直接重复添加'));
      this.pending.resolve = null; this.pending.reject = null;
    }
    this.onState(this.pending ? 'Space 已重载，请核对原请求；重新连接后继续' : '正在连接完整 Space…',!!this.pending);
    clearTimeout(this.readyTimer);
    this.readyTimer = setTimeout(() => {
      if (!this.ready && this.origin) this.onState(`${this.origin}/space/ 未响应上下文桥；检查服务状态或重新连接。服务接口错误由 Space 显示`,true);
    },15000);
    if (this.origin && this.frame.getAttribute('src')) this.hello();
  }
  reset(origin) {
    clearTimeout(this.readyTimer);
    if (this.query) { clearTimeout(this.query.timer); this.query.reject(new Error('服务或文档已变化，本次采集取消')); }
    if (this.pending) { clearTimeout(this.pending.timer); this.pending.reject?.(new Error('服务或文档已变化；本次采集副本保留，原请求结果请在原会话核对')); }
    this.query = null; this.pending = null; this.ready = false; this.origin = origin;
  }
  hello() { this.frame.contentWindow?.postMessage({protocol,type:'hello',requestId:'hello'},this.origin); }
  receive(event) {
    if (event.source !== this.frame.contentWindow || event.origin !== this.origin || event.data?.protocol !== protocol) return;
    const value = event.data;
    if (value.type === 'ready') {
      this.ready = true; clearTimeout(this.readyTimer);
      this.onState(this.pending?.invalidated ? 'Space 已重载，请核对原请求；重新连接后继续' : this.pending ? '原请求待确认，请核对或重试原请求' : 'Space 已连接；在 Space 内选择成员后采集',!!this.pending?.invalidated); return;
    }
    if (this.query && value.requestId === this.query.requestId && ['target','result'].includes(value.type)) {
      const query = this.query; clearTimeout(query.timer); this.query = null;
      if (value.type === 'target' && value.status === 'selected' && typeof value.selectionRevision === 'string' && value.selectionRevision && typeof value.target?.scope === 'string' && typeof value.target?.generation === 'string' && typeof value.target?.agent?.name === 'string' && typeof value.target?.agent?.incarnation === 'string' && value.target.agent.stopped_event === null) query.resolve(value);
      else query.reject(new Error(value.type !== 'target' ? `此服务尚不支持当前成员上下文桥；请使用更新后的 Space（${value.error || '未知响应'}）` : value.error || '当前 Space 无可添加上下文的成员；请先在 Space 内选择当前成员'));
      return;
    }
    if (value.type !== 'result' || !this.pending || value.requestId !== this.pending.envelope.requestId || !['added','error'].includes(value.status)) return;
    const pending = this.pending; clearTimeout(pending.timer); this.pending = null;
    if (value.status === 'added') {
      if (!sameTarget(value.target,pending.envelope.target) || value.selectionRevision !== pending.envelope.selectionRevision) {
        pending.reject?.(new Error('Space 返回的接收目标与本次采集不一致；请核对草稿，勿重复添加')); this.onState('接收目标响应不一致，请核对草稿',true); return;
      }
      pending.resolve?.(value); this.onState(`已加入 ${value.target.agent.name} 草稿；请在 Space 内检查并发送`);
    } else {
      const error = new Error(value.error || 'Space 拒绝了上下文；本次采集副本保留'); pending.reject?.(error); this.onState(error.message,true);
    }
  }
  selection() {
    if (!this.ready || this.query || this.pending) return Promise.reject(new Error('Space 尚未就绪或上一请求未结束'));
    return new Promise((resolve,reject) => {
      const requestId = crypto.randomUUID();
      const timer = setTimeout(() => { this.query = null; reject(new Error('Space 当前选择查询超时；请检查服务后重新采集')); },10000);
      this.query = {requestId,resolve,reject,timer};
      this.frame.contentWindow.postMessage({protocol,type:'getTarget',requestId},this.origin);
    });
  }
  send(selection,context) {
    if (!this.ready || this.pending) return Promise.reject(new Error('Space 尚未就绪或原请求待确认'));
    this.pending = {envelope:{protocol,type:'addContext',requestId:crypto.randomUUID(),target:selection.target,selectionRevision:selection.selectionRevision,context}};
    return this.dispatch();
  }
  dispatch() {
    return new Promise((resolve,reject) => {
      const pending = this.pending;
      if (!pending || pending.invalidated || !this.ready) { reject(new Error('原请求已结束或去重文档已变化，请核对草稿')); return; }
      pending.resolve = resolve; pending.reject = reject; clearTimeout(pending.timer);
      pending.timer = setTimeout(() => {
        pending.resolve = null; pending.reject = null;
        reject(new Error('结果尚未确认；保留本次采集副本，可重试原请求，勿重新采集后重复添加'));
        this.onState('结果尚未确认；可核对草稿或重试原请求',true);
      },30000);
      this.frame.contentWindow.postMessage(pending.envelope,this.origin);
    });
  }
}
