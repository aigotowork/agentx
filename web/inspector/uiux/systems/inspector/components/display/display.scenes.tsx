/** @aiPurpose Demonstrate real canonical components with local state and explicit event receipts. */
import {useState} from 'react';
import type {SceneProps} from '../../../../framework/framework.public';
import {Button,IconButton,ChoiceGroup,TextField,SearchField,Panel,Badge,Switch,Sheet,KeyValue,ViewTabs,TrendChart} from '../../inspector.native';
import {useDesignSystem} from '../../platforms/web/studio.context';
function Demo({emit}:SceneProps){const {theme}=useDesignSystem();const [value,setValue]=useState(false);return <div className="story-grid"><KeyValue theme={theme} label="已知样例" value="488" compact /><Switch theme={theme} label="目录受控开关" value={value} onChange={next=>{setValue(next);emit('change',{value:next});}} /></div>;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:Demo};
