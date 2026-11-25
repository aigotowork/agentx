import { cloudTokenPattern } from '@agentx/client';
import { useEffect, useState } from 'react';

export type TokenRetention = 0 | 30 | 90;
const day = 86_400_000;
const key = (audience: string) => 'agentx.cloud.login.' + audience;

export function forgetLogin(audience: string) {
  try { localStorage.removeItem(key(audience)); } catch { /* Storage may be disabled. */ }
}

export function rememberLogin(audience: string, token: string, days: TokenRetention) {
  if (!days) return forgetLogin(audience);
  try { localStorage.setItem(key(audience), JSON.stringify({ token, expires: Date.now() + days * day })); } catch { /* Page-only session. */ }
}

export function useRememberedLogin<T>(audience: string, validate: (token: string, signal: AbortSignal) => Promise<T>, connected: (session: T) => void) {
  const [restoring, setRestoring] = useState(false);
  useEffect(() => {
    const request = new AbortController();
    setRestoring(false);
    try {
      const saved = JSON.parse(localStorage.getItem(key(audience)) ?? 'null');
      if (!saved) return;
      if (!Number.isFinite(saved.expires) || saved.expires <= Date.now() || typeof saved.token !== 'string' || !cloudTokenPattern.test(saved.token)) {
        forgetLogin(audience);
        return;
      }
      setRestoring(true);
      void validate(saved.token, request.signal).then(session => {
        if (!request.signal.aborted) connected(session);
      }).catch(() => {
        // Always leave manual login available after a rejected or interrupted restore.
      }).finally(() => { if (!request.signal.aborted) setRestoring(false); });
    } catch { /* Missing or unreadable storage falls back to the login form. */ }
    return () => request.abort();
  }, [audience, validate, connected]);
  return restoring;
}
