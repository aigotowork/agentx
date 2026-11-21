/** Spatial contracts independently consumed without a DOM or playback clock. */
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
const root = new URL('../',import.meta.url), context=vm.createContext({});
for(const name of ['core.js','viewer/ports.js','viewer/canvas.js'])vm.runInContext(await readFile(new URL('tools/flow-atlas/'+name,root),'utf8'),context);
for(const name of ['tools/flow-atlas/examples/request.json','.docs/architecture/runtime-atlas/model.json']){
 const model=JSON.parse(await readFile(new URL(name,root),'utf8'));
 const before=JSON.stringify(model), {FlowAtlas:core,FlowAtlasCanvas:canvas}=context;
 for(const mode of model.modes){
  const boxes={};mode.nodeIds.forEach((id,index)=>{const node=model.nodes.find(n=>n.id===id);boxes[id]=canvas.box(model,node,{x:index*290,y:42});});
  const anchors=canvas.pins(model,mode,{},boxes);
  for(const scenario of model.scenarios.filter(s=>s.modeId===mode.id))for(let i=0;i<scenario.steps.length;i++){
   for(const call of core.calls(model,scenario.id,i)){
    const path=canvas.link(call,boxes,anchors,0),pin=anchors[call.port.id],box=boxes[call.target.id];
    assert.equal(path.end.x,pin.x);assert.equal(path.end.y,pin.y);
    assert(pin.x===box.x||pin.x===box.x+box.width);assert(pin.y>box.y&&pin.y<box.y+box.height);
    assert.match(canvas.payload(call,true),/待确认/);
    assert(!canvas.payload(call,true).includes(call.result.summary)||call.result.summary==='');
   }
  }
 }
 assert.equal(JSON.stringify(model),before,'Geometry never mutates canonical data');
}
console.log('PASS: named Port anchors, exact caller→Port geometry, pending reply payload and pure projection');
