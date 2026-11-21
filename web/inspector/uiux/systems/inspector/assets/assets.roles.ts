/** @aiPurpose Map product-independent asset intent to owned SVG, CSS background and optional sound resources. */
import {toolImage} from './tools/tools.assets';
export const assetRoles={
 'tool.confirm':{kind:'svg',origin:'原创整数网格矢量',license:'MIT'},
 'tool.explore':{kind:'svg',origin:'原创整数网格矢量',license:'MIT'},
 'tool.refine':{kind:'svg',origin:'原创整数网格矢量',license:'MIT'},
 'background.workspace':{kind:'css',origin:'共享主题纹理',license:'MIT'},
 'background.investigation':{kind:'svg',origin:'原创阶梯图案',license:'MIT'},
 'feedback.tap':{kind:'sound',origin:'本地合成短敲',license:'MIT'},
} as const;
/** @aiPurpose Select owned resources by semantic meaning. @aiUsage Consumers request a role; SVG/data URLs and texture variables stay system-owned. */
export function resolveAsset(role:keyof typeof assetRoles,style:'original'|'pixel'='pixel'):string {
 if(role==='tool.confirm')return toolImage('hammer');if(role==='tool.explore')return toolImage('shovel');if(role==='tool.refine')return toolImage('chisel');
 if(role==='background.workspace')return style==='pixel'?'var(--ds-background-grid)':'none';
 if(role==='background.investigation')return new URL('./backgrounds/investigation.svg',import.meta.url).href;
 return style==='pixel'?'wood':'metal';
}
