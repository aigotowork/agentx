/** @aiPurpose Demonstrate real canonical components with local state and explicit event receipts. */
import {useState} from 'react';
import type {SceneProps} from '../../../../framework/framework.public';
import {Button,IconButton,ChoiceGroup,TextField,SearchField,Panel,Badge,Switch,Sheet,KeyValue,ViewTabs,TrendChart} from '../../inspector.native';
import {useDesignSystem} from '../../platforms/web/studio.context';
function Demo({emit}:SceneProps){const {theme}=useDesignSystem();const [open,setOpen]=useState(false);return <><Button theme={theme} label="打开目录示例 Sheet" onPress={()=>{setOpen(true);emit('open');}} /><Sheet title="目录示例" open={open} onClose={()=>{setOpen(false);emit('close');}}><Button theme={theme} label="关闭目录示例" onPress={()=>{setOpen(false);emit('close');}} /></Sheet></>;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:Demo};
