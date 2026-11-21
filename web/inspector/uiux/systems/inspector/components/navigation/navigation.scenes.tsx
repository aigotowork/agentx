/** @aiPurpose Demonstrate real canonical components with local state and explicit event receipts. */
import {useState} from 'react';
import type {SceneProps} from '../../../../framework/framework.public';
import {Button,IconButton,ChoiceGroup,TextField,SearchField,Panel,Badge,Switch,Sheet,KeyValue,ViewTabs,TrendChart} from '../../inspector.native';
import {useDesignSystem} from '../../platforms/web/studio.context';
function Demo({emit}:SceneProps){const {theme}=useDesignSystem();const [value,setValue]=useState('overview');return <ViewTabs theme={theme} value={value} onChange={next=>{setValue(next);emit('change',{value:next});}} tabs={[{value:'overview',label:'总览',icon:'Layers',colors:['blue','cyan']},{value:'detail',label:'详情',icon:'Folder',colors:['green','teal']}]} />;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:Demo};
