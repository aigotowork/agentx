import {offices} from './fixtures.js';

const W=900, H=600;
const minFactor=.88, maxFactor=3.5;
const clamp = (n, lo, hi) => Math.min(hi, Math.max(lo,n));

/** Page-owned cameras stored in world coordinates; resizing preserves center and zoom ratio. */
export function setupScene(view, plan, markers, label) {
  let office = '', drag, reveal, camera;
  const memory = new Map();
  function base() {
    if (!office || office === ':unknown') {
      return {x:W/2,y:H/2,scale:Math.max(.48,Math.min(view.clientWidth/W, view.clientHeight/H))};
    }
    const selected = offices.find(o=>o.id===office);
    const wideRoom = selected.code==='01'||selected.code==='02';
    return {x:selected.x*9,y:wideRoom?230:453,scale:Math.max(.62,Math.min(1.75, view.clientWidth/(wideRoom?430:350), view.clientHeight/(wideRoom?315:220)))};
  }
  function project() {
    if (!view.clientWidth || !view.clientHeight) return;
    const b = base(), {x,y,factor} = camera;
    const scale = b.scale*factor;
    const left = view.clientWidth/2-x*scale, top = view.clientHeight/2-y*scale;
    plan.style.transform = `translate(${left}px, ${top}px) scale(${scale})`;
    const crowded = scale < .69;
    const scenePeople = Array.from(markers.querySelectorAll('.map-actor'));
    const visibleSet = new Set(scenePeople.filter(node=>node.dataset.selected==='true' || !crowded));
    scenePeople.forEach(node=>{node.hidden=!visibleSet.has(node);});
    for (const node of markers.children) {
      const dx = left+Number(node.dataset.x)*9*scale, dy=top+Number(node.dataset.y)*6*scale;
      node.style.left = dx+'px';
      node.style.top = dy+'px';
      node.toggleAttribute('inert', dx<14 || dy<14 || dx>view.clientWidth-14 || dy>view.clientHeight-14);
    }
    label.textContent = Math.round(factor*100)+'%';
    view.dataset.crowded = String(crowded);
  }
  function frame() {
    const b = base();
    return {x: b.x, y:b.y, factor:1};
  }
  function selectOffice(value) {
    if (camera && office===value) return;
    office=value;
    camera=memory.get(office) || frame();
    memory.set(office,camera);
    reveal = null;
    project();
  }
  function reset() {
    Object.assign(camera,frame());
    reveal=null;
    project();
  }
  function bringIntoView(agent, remember = true) {
    if (agent?.x == null || !view.clientWidth || !view.clientHeight) return;
    const scale=base().scale*camera.factor;
    const nw=agent.x*9, nh=agent.y*6;
    const px=view.clientWidth/2+(nw-camera.x)*scale, py=view.clientHeight/2+(nh-camera.y)*scale;
    const insetX=Math.min(62,view.clientWidth/3), insetY=Math.min(56,view.clientHeight/3);
    if(remember && !reveal) reveal={key:agent.key,x:camera.x,y:camera.y};
    camera.x+=(px-clamp(px,insetX,view.clientWidth-insetX))/scale;
    camera.y+=(py-clamp(py,insetY,view.clientHeight-insetY))/scale;
    project();
  }
  function unreveal() {
    if (reveal) {
      camera.x=reveal.x; camera.y=reveal.y;
      reveal=null;
      project();
    }
  }
  function zoom(delta) {
    reveal=null;
    camera.factor=clamp(camera.factor*(delta>0?1.25:.8),minFactor,maxFactor);
    project();
  }
  function move(dx,dy) {
    reveal=null;
    const scale=base().scale*camera.factor;
    camera.x=clamp(camera.x-dx/scale,-80,W+80); camera.y=clamp(camera.y-dy/scale,-60,H+60);
    project();
  }
  view.addEventListener('pointerdown',e=>{
    if (e.button!==0 || e.target.closest('button, aside')) return;
    drag={id:e.pointerId,x:e.clientX,y:e.clientY};
    view.setPointerCapture(e.pointerId);
    view.dataset.panning='true';
  });
  view.addEventListener('pointermove',e=>{
    if (drag?.id!==e.pointerId) return;
    move(e.clientX-drag.x,e.clientY-drag.y);
    drag.x=e.clientX; drag.y=e.clientY;
  });
  for(const type of ['lostpointercapture','pointercancel','pointerup']) view.addEventListener(type,()=>{drag=null;delete view.dataset.panning;});
  view.addEventListener('keydown',e=>{
    if(e.target!==view)return;
    const direction={ArrowLeft:[38,0],ArrowRight:[-38,0],ArrowUp:[0,38],ArrowDown:[0,-38]}[e.key];
    if(!direction)return;
    e.preventDefault();
    move(...direction);
  });
  new ResizeObserver(project).observe(view);
  selectOffice('');
  return {project,selectOffice,bringIntoView,unreveal,zoom,reset,move};
}
