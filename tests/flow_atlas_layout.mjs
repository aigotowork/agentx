/** Layout ownership, identity, gesture transactions and explicit routing failure. */
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile} from 'node:fs/promises';
import vm from 'node:vm';
const context=vm.createContext({});vm.runInContext(await readFile(new URL('../tools/flow-atlas/viewer/layout.js',import.meta.url),'utf8'),context);
const L=context.FlowAtlasLayout,plain=value=>JSON.parse(JSON.stringify(value));
const model={label:'模型',a:[1,{b:2}],z:false};
const canonical=value=>value&&typeof value==='object'?(Array.isArray(value)?'['+value.map(canonical).join(',')+']':'{'+Object.keys(value).sort().map(key=>JSON.stringify(key)+':'+canonical(value[key])).join(',')+'}'):JSON.stringify(value);
assert.equal(L.identity(model),'sha256:'+createHash('sha256').update(canonical(model)).digest('hex'));
assert.equal(L.identity(model),L.identity({z:false,a:[1,{b:2}],label:'模型'}));
const options={contentDigest:L.identity(model),scope:{view:'relations',modeId:'main',scenarioId:'start'},positions:{a:{x:10,y:20},b:{x:320,y:20}}};
const before=JSON.stringify(options),original=L.create(options);let session=L.begin(original,'a');
for(let i=0;i<30;i++)session=L.update(session,{x:20+i,y:30+i});
assert.equal(session.history.length,0);assert.deepEqual(plain(L.cancel(session).document),plain(original.document));
session=L.commit(session);assert.equal(session.history.length,1);assert.deepEqual(plain(L.undo(session).document),plain(original.document));
assert.equal(L.commit(L.begin(original,'a')).history.length,0);
const pinned=L.togglePin(session,'a');assert.equal(L.move(pinned,'a',{x:900,y:900}).document.positions.a.x,49);
const exported=L.exportDocument(session);assert.equal(exported.contentDigest,options.contentDigest);assert.equal(exported.scope.scenarioId,'start');
const imported=L.importDocument(original,exported);assert.deepEqual(plain(imported.document),plain(session.document));assert.equal(imported.history.length,1);
for(const change of [d=>d.contentDigest=L.identity({other:true}),d=>d.scope.scenarioId='elsewhere',d=>d.positions.a.x=Infinity,d=>d.positions.extra={x:0,y:0},d=>d.pinned=['a','a'],d=>d.extra=true]){
 const candidate=plain(exported);change(candidate);assert.throws(()=>L.importDocument(session,candidate));
 assert.deepEqual(plain(session.document),plain(exported));
}
const boxes={a:{x:0,y:0,width:100,height:100},b:{x:200,y:0,width:100,height:100},block:{x:130,y:0,width:20,height:100}};
assert.equal(L.routeDiagnostics('test',[{x:100,y:50},{x:200,y:50}],boxes)[0].code,'route-obstructed');
assert.equal(L.routeDiagnostics('test',[{x:100,y:50},{x:120,y:50},{x:120,y:120},{x:200,y:120},{x:200,y:50}],boxes).length,0);
assert.equal(L.routeDiagnostics('test',[],boxes)[0].code,'route-unresolved');
assert.equal(L.overlaps({a:boxes.a,b:{x:50,y:50,width:100,height:100}}).length,1);
assert.equal(JSON.stringify(options),before);
console.log('PASS: identity, immutable gesture/cancel/undo/pin, strict atomic exchange and obstruction diagnostics');
