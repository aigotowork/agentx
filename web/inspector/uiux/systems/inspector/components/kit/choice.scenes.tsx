/** @aiPurpose Demonstrate real canonical components with local state and explicit event receipts. */
import {useState} from 'react';
import type {SceneProps} from '../../../../framework/framework.public';
import {Button,IconButton,ChoiceGroup,TextField,SearchField,Panel,Badge,Switch,Sheet,KeyValue,ViewTabs,TrendChart} from '../../inspector.native';
import {useDesignSystem} from '../../platforms/web/studio.context';
function Demo({emit}:SceneProps){const {theme}=useDesignSystem();const [value,setValue]=useState('all');return <ChoiceGroup theme={theme} label="资源选择" layout="wrap" value={value} onChange={next=>{setValue(next);emit('change',{value:next});}} options={[{value:'all',label:'全部'},{value:'pending',label:'待核实'},{value:'blocked',label:'不可选',disabled:true}]} />;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:Demo};
