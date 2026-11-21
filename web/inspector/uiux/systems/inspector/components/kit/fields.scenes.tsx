/** @aiPurpose Demonstrate real canonical components with local state and explicit event receipts. */
import {useState} from 'react';
import type {SceneProps} from '../../../../framework/framework.public';
import {Button,IconButton,ChoiceGroup,TextField,SearchField,Panel,Badge,Switch,Sheet,KeyValue,ViewTabs,TrendChart} from '../../inspector.native';
import {useDesignSystem} from '../../platforms/web/studio.context';
function Demo({emit}:SceneProps){const {theme}=useDesignSystem();const [value,setValue]=useState('');return <div className="story-grid"><TextField theme={theme} label="目录草稿" value={value} onChangeText={text=>{setValue(text);emit('change',{value:text});}} /><SearchField theme={theme} value={value} onChangeText={text=>{setValue(text);emit('change',{value:text});}} grow={false} /></div>;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:Demo};
