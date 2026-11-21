import { useEffect, useRef, useState, type FormEvent, type ReactNode } from 'react';
import { Icon } from '../presentation';
import { cloudTokenPattern, deploymentClient, explainError, type AdminClient, type AdminOverview } from './model';
import { DeploymentWorkspace } from './workspace';
import './deployment.css';

function DeploymentHeader({ children }: { children?: ReactNode }) {
  return <header className="app-header deployment-header">
    <div className="brand">
      <span className="brand-symbol"><Icon name="route" /></span>
      <strong>AgentX</strong><span className="deployment-product">云端部署</span>
    </div>
    {children}
  </header>;
}

type Session = { client: AdminClient; overview: AdminOverview };

function DeploymentGate({ onConnect }: { onConnect: (session: Session) => void }) {
  const [token, setToken] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const pending = useRef<AbortController | null>(null);
  useEffect(() => () => pending.current?.abort(), []);

  async function connect(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const bearer = token.trim();
    if (pending.current || !bearer) return;
    if (!cloudTokenPattern.test(bearer)) { setError('请输入 32–256 个 URL 安全字符的管理员令牌。'); return; }
    const controller = new AbortController();
    pending.current = controller;
    setError('');
    setBusy(true);
    try {
      const client = deploymentClient(bearer);
      const overview = await client.overview({ signal: controller.signal });
      if (!controller.signal.aborted) onConnect({ client, overview });
    } catch (failure) {
      if (!controller.signal.aborted) setError(explainError(failure, '无法连接部署。请确认部署地址和网络后重试。'));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
      pending.current = null;
    }
  }

  return <main className="deployment-gate">
    <form className="deployment-signin" aria-labelledby="deployment-title" autoComplete="off" onSubmit={connect}>
      <h1 id="deployment-title">连接云端部署</h1>
      <p className="deployment-origin"><code>{location.origin}</code></p>
      {isSecureContext ? <>
        <label>管理员令牌<input type="password" value={token} required autoComplete="off" spellCheck={false} readOnly={busy} aria-invalid={!!error} onChange={event => { setToken(event.target.value); setError(''); }} /></label>
        {error && <p role="alert" className="deployment-error">{error}</p>}
        <button className="primary" type="submit" disabled={busy || !token.trim()}>{busy ? '正在连接…' : '连接'}</button>
      </> : <p role="alert" className="deployment-error">请通过 HTTPS 打开部署管理台。</p>}
    </form>
  </main>;
}

/**
 * @aiPurpose Mount the deployment operator workspace on the cloud listener.
 * @aiUsage Render only for the /deployment entry; the console owns its in-memory admin session.
 */
export function DeploymentConsole() {
  const [session, setSession] = useState<Session | null>(null);
  useEffect(() => {
    const resetRestoredPage = (event: PageTransitionEvent) => { if (event.persisted) setSession(null); };
    window.addEventListener('pageshow', resetRestoredPage);
    return () => window.removeEventListener('pageshow', resetRestoredPage);
  }, []);

  return <div className="deployment-app">
    <DeploymentHeader>{session && <button className="quiet deployment-disconnect" onClick={() => setSession(null)}>断开</button>}</DeploymentHeader>
    {session ? <DeploymentWorkspace client={session.client} initialOverview={session.overview} /> : <DeploymentGate onConnect={setSession} />}
  </div>;
}
