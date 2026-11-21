/** @aiPurpose Inspect the legacy DOM system without changing its product contract or global theme. */
import {createContext,useContext,useState,type ReactNode} from 'react';
import {Button,Field} from '@longwork/uiux/workbench';
const Context=createContext({dark:false,accent:'#315f46',setDark:(_value:boolean)=>{},setAccent:(_value:string)=>{}});
/** @aiPurpose Compose Provider presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Provider({children}:{children:ReactNode}){const [dark,setDark]=useState(false),[accent,setAccent]=useState('#315f46');return <Context.Provider value={{dark,setDark,accent,setAccent}}>{children}</Context.Provider>;}
/** @aiPurpose Provide useRecoveryPreview presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useRecoveryPreview(){return useContext(Context);}
/** @aiPurpose Compose Controls presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Controls(){const {dark,setDark}=useRecoveryPreview();return <Button variant="quiet" onClick={()=>setDark(!dark)}>{dark?'经典深色':'经典浅色'}</Button>;}
/** @aiPurpose Compose TokenEditor presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function TokenEditor(){const {accent,setAccent}=useRecoveryPreview();return <Field label="强调色"><input aria-label="恢复系统强调色" type="color" value={accent} onChange={event=>setAccent(event.target.value)}/></Field>;}
/** @aiPurpose Provide validateSettings presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function validateSettings(value:unknown):string|null{if(!value||typeof value!=='object'||Array.isArray(value))return '经典主题方案无效';const data=value as Record<string,unknown>;return Object.keys(data).some(key=>!['dark','accent'].includes(key))||typeof data.dark!=='boolean'||typeof data.accent!=='string'||!/^#[0-9a-f]{6}$/i.test(data.accent)?'经典主题方案无效':null;}
/** @aiPurpose Provide useSettings presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useSettings(){const {dark,setDark,accent,setAccent}=useRecoveryPreview();return {value:{dark,accent},apply:(value:Record<string,unknown>)=>{if(validateSettings(value))throw new Error('无效主题方案');setDark(value.dark as boolean);setAccent(value.accent as string);}};}
