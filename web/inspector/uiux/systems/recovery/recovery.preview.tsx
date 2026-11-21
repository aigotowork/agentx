/** @aiPurpose Adapt the original scoped CSS to an isolated design surface; render real components through a React portal. */
import {useEffect,useState,type ReactNode,type CSSProperties} from 'react';import {createPortal} from 'react-dom';
import tokens from './foundation.tokens.css?raw';import styles from './kit.styles.css?raw';import {useRecoveryPreview} from './recovery.adapter';
const css=(tokens+'\n'+styles).replaceAll(':root[data-experience="recovery"]',':host')+'\n:host{display:block;font:var(--text)/var(--lh) var(--font);color:var(--ink);background:var(--surface);padding:var(--s2)}.preview-stack{display:flex;flex-direction:column;gap:12px}.preview-row{display:flex;align-items:center;flex-wrap:wrap;gap:8px}';
/** @aiPurpose Compose RecoveryPreview presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function RecoveryPreview({children}:{children:ReactNode}){const [host,setHost]=useState<HTMLDivElement|null>(null),[root,setRoot]=useState<ShadowRoot|null>(null);const {dark,accent}=useRecoveryPreview();useEffect(()=>{if(host)setRoot(host.shadowRoot??host.attachShadow({mode:'open'}));},[host]);return <div ref={setHost} data-theme={dark?'dark':'light'} style={{'--accent':accent} as CSSProperties}>{root&&createPortal(<><style>{css}</style>{children}</>,root)}</div>;}
