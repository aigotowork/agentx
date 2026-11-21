/** Three-layer observation for the real sidePanel omitted by Playwright's page list. */
import {mkdir,writeFile} from 'node:fs/promises';
export async function extensionCheckpoint(panel,directory,id,meaning,nativeState=null){
  const output=`${directory}/${id}`;await mkdir(output);
  const started=new Date().toISOString();
  const cdp=await panel.createCDPSession();
  const semantic=await cdp.send('Accessibility.getFullAXTree');await cdp.detach();
  const state=await panel.evaluate(()=>({url:location.href,viewport:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
    focus:document.activeElement?.outerHTML,body:document.body.innerText,scrollWidth:document.documentElement.scrollWidth,
    controls:[...document.querySelectorAll('button,input,select,textarea,summary,iframe')].map(element=>{const bounds=element.getBoundingClientRect();return{tag:element.tagName,id:element.id,label:element.getAttribute('aria-label'),text:element.innerText,disabled:element.disabled,hidden:element.hidden,bounds:{x:bounds.x,y:bounds.y,width:bounds.width,height:bounds.height}};})}));
  await writeFile(output+'/semantic.json',JSON.stringify(semantic,null,2));await writeFile(output+'/state.json',JSON.stringify(state,null,2));
  await panel.screenshot({path:output+'/viewport.png'});
  await writeFile(output+'/checkpoint.json',JSON.stringify({id,meaning,started,finished:new Date().toISOString(),nativeState,
    limits:'Sequential actual target captures, not atomic; no private React state. AX tree belongs to parent panel; child semantic/draft state is sampled separately.'},null,2));
}
