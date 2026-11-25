import { cloudTokenPattern } from '@agentx/client';
import { forgetLogin, rememberLogin, useRememberedLogin, type TokenRetention } from '../../../management/deployment/remember';
import { useCallback, useEffect, useState, useRef, type FormEvent } from 'react';
import { SpaceTheme } from '../../uiux/systems/space/space.web';
import { App } from '../App';
import { OfficeLayout } from '../../uiux/systems/space/layouts/OfficeLayout';
import { readAppearance } from '../navigation/appearance';
import { cloudProblem, connectCloud, type Session } from './model';
import { CloudWorkbench } from './workbench';
import { accessHint, validNamespace, type Access } from './route';
import './cloud.css';

const permissions: Record<Access, string> = { admin: '清单、读取与发送消息、启动、停止和回执', namespace: '清单与只读云端消息' };

/** @aiPurpose Admit one Namespace bearer before mounting the shared Space workbench.
 * @aiUsage Mount only for /namespaces/{namespace}/space/; credentials belong to this gate.
 */
export function CloudSpace({ namespace }: { namespace: string }) {
  const [session, setSession] = useState<Session>();
  useEffect(() => {
    function restore(event: PageTransitionEvent) { if (event.persisted) setSession(undefined); }
    window.addEventListener('pageshow', restore);
    return () => window.removeEventListener('pageshow', restore);
  }, []);
  function disconnect() {
    if (session) forgetLogin(session.access === 'admin' ? 'admin' : namespaceAudience(namespace));
    setSession(undefined);
  }
  if (session) return <App cloud={publish => new CloudWorkbench(session, publish)} onCloudDisconnect={disconnect} />;
  const header = <><div className="office-brand"><strong>云端 Space</strong> <code>{namespace}</code></div><a className="cloud-deployment-link" href="/deployment">部署管理</a></>;
  return <SpaceTheme style={readAppearance(false).style}><OfficeLayout header={header} context={null}>
    {validNamespace(namespace) ? <Connect namespace={namespace} onConnect={setSession} /> : <p role="alert">Namespace 标识无效。</p>}
  </OfficeLayout></SpaceTheme>;
}

const namespaceAudience = (namespace: string) => 'namespace.' + namespace;

function Connect({ namespace, onConnect }: { namespace: string; onConnect(session: Session): void }) {
  const [access, setAccess] = useState(() => accessHint(location.search));
  const [token, setToken] = useState('');
  const [retention, setRetention] = useState<TokenRetention>(90);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const audience = access === 'admin' ? 'admin' : namespaceAudience(namespace);
  const validate = useCallback((token: string, signal: AbortSignal) => connectCloud(namespace, access, token, signal), [namespace, access]);
  const restoring = useRememberedLogin(audience, validate, onConnect);
  const controller = useRef<AbortController>(null);
  useEffect(() => () => controller.current?.abort(), []);
  async function connect(event: FormEvent) {
    event.preventDefault();
    if (controller.current) return;
    const bearer = token;
    if (!cloudTokenPattern.test(bearer)) { setError('请输入 12 位字母数字令牌。'); return; }
    const request = new AbortController();
    controller.current = request;
    setBusy(true);
    setError('');
    try {
      const session = await connectCloud(namespace, access, bearer, request.signal);
      if (!request.signal.aborted) { rememberLogin(audience, bearer, retention);
        const url = new URL(location.href);
        url.searchParams.set('access', access);
        history.replaceState(history.state, '', url); onConnect(session); }
    } catch (failure) {
      if (!request.signal.aborted) setError(cloudProblem(failure, '无法连接云端 Space，请重试。'));
    } finally {
      controller.current = null;
      if (!request.signal.aborted) setBusy(false);
    }
  }
  return <div className="cloud-entry"><form className="cloud-connect" onSubmit={connect} autoComplete="off">
    <div className="cloud-connect-mark" aria-hidden="true"><svg viewBox="0 0 32 32" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="m3 14 13-8 13 8v12H3zM12 26v-9h8v9M1 15 16 5l15 10" /></svg></div>
    <h2>连接此 Namespace</h2>
    <label>访问方式<select value={access} disabled={busy || restoring} onChange={event => { const next = event.target.value as Access; setAccess(next); setToken(''); setError(''); }}>
      <option value="namespace">Namespace 令牌</option><option value="admin">管理员令牌</option>
    </select></label>
    <p className="cloud-note">{permissions[access]}</p>
    <label>访问令牌<input value={token} onChange={event => setToken(event.target.value)} type="password" required autoComplete="off" spellCheck={false} aria-describedby={error ? 'cloud-connect-error' : undefined} aria-invalid={!!error} /></label>
    <label>记住令牌<select value={retention} onChange={event => setRetention(Number(event.target.value) as TokenRetention)} disabled={busy || restoring}>
      <option value="0">仅本次</option><option value="30">30 天</option><option value="90">90 天</option>
    </select></label>
    {error && <p id="cloud-connect-error" role="alert">{error}</p>}
    <button className="cloud-primary" type="submit" disabled={busy || restoring}>{busy ? '正在连接…' : '连接'}</button>
  </form></div>;
}
