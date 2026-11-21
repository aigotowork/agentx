/** @aiPurpose Adapt Inspector themes and editing controls to the product-neutral uiux workbench. */
import {validateInspectorSettings} from './themes/experiment.model';
import {useState} from 'react';
import {Button,IconButton,Segmented,Field,Sheet,Icon} from '@longwork/uiux/workbench';
import {SystemProvider,useDesignSystem} from './platforms/web/studio.context';
import {inspectorTokens} from './themes/theme.resolve';import type {TokenValues} from '../../framework/framework.public';
/** @aiPurpose Provide tokens presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const tokens=inspectorTokens;
/** @aiPurpose Provide useTokens presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useTokens(){const state=useDesignSystem();return {mode:state.appearance.style+'.'+state.appearance.theme,values:{...state.tokens,...state.overrides} as TokenValues,apply:(values:TokenValues)=>{state.setTokens(values);state.setOverrides({});}};}
import './platforms/web/host.styles.css';
import './boards/review/review.styles.css';
import './platforms/web/appearance.styles.css';
import './motion/feedback.styles.css';
import '../../../web/client/host/i18n';
export {SystemProvider as Provider};
/** @aiPurpose Offer the system's real appearance and feedback controls. @aiUsage The workbench mounts this slot under the same Provider as scenes. */
/** @aiPurpose Compose Controls presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Controls(){const {appearance,setAppearance,feedback}=useDesignSystem();return <div className="wb-cluster"><Segmented label="预览设计风格" value={appearance.style} onChange={style=>setAppearance(current=>({...current,style:style as 'original'|'pixel'}))} options={[{value:'original',label:'原版'},{value:'pixel',label:'像素'}]}/><IconButton icon={appearance.theme==='light'?'sun':'moon'} label={`预览主题：${appearance.theme==='light'?'浅色':'深色'}`} onClick={()=>setAppearance(current=>({...current,theme:current.theme==='light'?'dark':'light'}))}/><IconButton icon={feedback.preferences.sound?'sound':'mute'} label={`预览声音：${feedback.preferences.sound?'开':'关'}`} active={feedback.preferences.sound} onClick={()=>feedback.setPreferences(current=>({...current,sound:!current.sound}))}/></div>}
function TokenRange({
  label,
  name,
  value,
  min,
  max,
  unit,
}: {
  label: string;
  name: "step" | "shadow" | "press" | "icon";
  value: number;
  min: number;
  max: number;
  unit: string;
}) {
  const { setOverrides } = useDesignSystem();
  return (
    <label className="token-field">
      <span>
        {label}
        <output>
          {value}
          {unit}
        </output>
      </span>
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={name === "step" ? 2 : 1}
        value={value}
        onChange={(event) =>
          setOverrides((current) => ({
            ...current,
            [name]: Number(event.target.value),
          }))
        }
      />
    </label>
  );
}
/** @aiPurpose Compose TokenEditor presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function TokenEditor(){const {resolved,theme,setOverrides,overrides}=useDesignSystem();const [transfer,setTransfer]=useState(''),[receipt,setReceipt]=useState(''),[open,setOpen]=useState(false);return <div className="wb-token-editor"><Field label="强调色"><input type="color" aria-label="强调色" value={theme.colors.accent} onChange={event=>setOverrides(current=>({...current,accent:event.target.value}))}/></Field><TokenRange label="阶梯切角" name="step" value={resolved.shape.step} min={2} max={10} unit="px"/><TokenRange label="硬阴影" name="shadow" value={resolved.shape.shadow} min={0} max={6} unit="px"/><TokenRange label="按下时长" name="press" value={resolved.motion.press} min={70} max={110} unit="ms"/><TokenRange label="图标微动" name="icon" value={resolved.motion.icon} min={120} max={400} unit="ms"/><div className="wb-cluster"><Button variant="quiet" onClick={()=>setOverrides({})}><Icon name="reset" size={14}/>重置变量</Button><IconButton icon="code" label="导入导出实验变量" onClick={()=>setOpen(true)}/></div><div className="wb-swatches">{(['surface0','surface1','accent','statusSuccess','statusWarning','statusDanger'] as const).map(role=><span key={role} title={role} style={{background:theme.colors[role]}}/>)}</div><Sheet open={open} onClose={()=>setOpen(false)} title="实验变量"><div className="wb-stack"><Button onClick={()=>setTransfer(JSON.stringify({schemaVersion:1,system:'inspector',overrides},null,2))}>生成实验 JSON</Button><textarea aria-label="实验变量 JSON" value={transfer} onChange={event=>setTransfer(event.target.value)} rows={12}/><Button variant="primary" onClick={()=>{try{const data=JSON.parse(transfer);if(data.schemaVersion!==1||data.system!=='inspector'||!data.overrides||typeof data.overrides!=='object'||Array.isArray(data.overrides))throw new Error('格式或系统不符');const allowed=['accent','step','shadow','press','icon','pressDepth','rhythm','chart'];for(const [name,value] of Object.entries(data.overrides))if(!allowed.includes(name)||(name==='accent'?typeof value!=='string'||!/^#[0-9a-f]{6}$/i.test(value):typeof value!=='number'||!Number.isFinite(value)))throw new Error('存在无效语义变量');setOverrides(data.overrides);setReceipt('已载入本地实验，源码未修改');}catch(error){setReceipt(error instanceof Error?error.message:'无效 JSON');}}}>载入本地实验</Button><output role="status">{receipt}</output></div></Sheet></div>;}
/** @aiPurpose Capture and apply only system-owned visual state for portable workspaces. */
/** @aiPurpose Provide useSettings presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useSettings(){const {appearance,setAppearance,overrides,setOverrides,tokens,setTokens}=useDesignSystem();return {value:{...appearance,overrides,tokens},apply:(value:Record<string,unknown>)=>{if(validateInspectorSettings(value))throw new Error('无效主题方案');setAppearance({style:value.style as 'original'|'pixel',theme:value.theme as 'light'|'dark'});setOverrides(value.overrides as import('./themes/theme.resolve').SystemOverrides);setTokens(value.tokens as TokenValues??{});}};}
export {validateInspectorSettings as validateSettings} from './themes/experiment.model';
