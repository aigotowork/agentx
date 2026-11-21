/** @aiPurpose Demonstrate real canonical components with local state and explicit event receipts. */
import {useState} from 'react';
import type {SceneProps} from '../../../../framework/framework.public';
import {Button,IconButton,ChoiceGroup,TextField,SearchField,Panel,Badge,Switch,Sheet,KeyValue,ViewTabs,TrendChart} from '../../inspector.native';
import {useDesignSystem} from '../../platforms/web/studio.context';
function Demo({emit}:SceneProps){const {theme}=useDesignSystem();return <div className="story-row"><IconButton theme={theme} icon="Search" label="搜索资源示例" onPress={()=>emit('press')} /><IconButton theme={theme} icon="Settings2" label="已禁用图标示例" disabled onPress={()=>emit('press')} /></div>;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:Demo};
