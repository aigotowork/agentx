/** @aiPurpose Demonstrate real canonical components with local state and explicit event receipts. */
import {useState} from 'react';
import type {SceneProps} from '../../../../framework/framework.public';
import {Button,IconButton,ChoiceGroup,TextField,SearchField,Panel,Badge,Switch,Sheet,KeyValue,ViewTabs,TrendChart} from '../../inspector.native';
import {useDesignSystem} from '../../platforms/web/studio.context';
function ButtonScene({args,emit}:SceneProps){const {theme}=useDesignSystem();const [count,setCount]=useState(0);return <div className="story-row"><Button theme={theme} label={String(args.label??'主操作')} variant={(args.variant??'primary') as 'primary'|'secondary'|'danger'|'ghost'} disabled={args.disabled===true} busy={args.busy===true} onPress={()=>{setCount(value=>value+1);emit('press',{count:count+1});}} /><output>本地计数 {count}</output></div>;}
function Matrix(props:SceneProps){const {theme}=useDesignSystem();return <div className="story-grid"><ButtonScene {...props}/><div className="story-row"><Button theme={theme} label="次操作" onPress={()=>props.emit('press',{variant:'secondary'})}/><Button theme={theme} label="危险操作" variant="danger" onPress={()=>props.emit('press',{variant:'danger'})}/><Button theme={theme} label="已禁用" disabled onPress={()=>{}}/><Button theme={theme} label="处理中" busy onPress={()=>{}}/><Button theme={theme} label="轻操作" variant="ghost" onPress={()=>props.emit('press',{variant:'ghost'})}/></div></div>;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:ButtonScene,disabled:ButtonScene,busy:ButtonScene,matrix:Matrix};
