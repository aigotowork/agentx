/** @aiPurpose Compose the genuine Panel with local controls; no domain requests. */
import {useState} from 'react';import {Panel,Badge,KeyValue,Button,Sheet} from '../../inspector.native';import {useDesignSystem} from '../../platforms/web/studio.context';
function Demo(){const {theme}=useDesignSystem();const [open,setOpen]=useState(false);return <><Panel theme={theme} title="观察摘要"><div className="story-row"><Badge theme={theme} label="本地样例" tone="info"/><Button theme={theme} label="查看详情" variant="ghost" onPress={()=>setOpen(true)}/></div></Panel><Sheet title="观察摘要样例" open={open} onClose={()=>setOpen(false)}><KeyValue compact theme={theme} label="资源" value="Panel"/><Button theme={theme} label="关闭" onPress={()=>setOpen(false)}/></Sheet></>;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:Demo};
