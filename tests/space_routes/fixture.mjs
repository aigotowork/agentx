/** Simulated native-shaped API supply; never a real AgentX service. */
export const scope = 'verification-scope', generation = '700';
export const ref = (name='mora', incarnation='101', stopped_event=null) => ({name, incarnation, stopped_event});
export const row = (agent=ref(), workspace='/fixture') => ({agent,title:'Simulated navigation fixture',file_bindings:[],pending_messages:0,tool:'codex',status:agent.stopped_event?'stopped':'listening',stored_status:'listening',status_context:'',request_error:null,detail:'Explicit simulated API response',directory:workspace??'/fixture',session_id:null,collaboration:{status:'adopted',workspace,role:null,reason:null},can_send:!agent.stopped_event,can_stop:false,can_stop_participation:false,can_fork:false,can_resume:false,has_transcript:false});
export function exactURL(base, agent=ref(), extras={}) {
 const url=new URL('/space/',base);for(const [k,v] of Object.entries({office:'/fixture',view:'chat',scope,generation,agent:agent.name,incarnation:agent.incarnation,stopped_event:agent.stopped_event,record:agent.stopped_event?'stopped':'current',...extras}))if(v!==null)url.searchParams.set(k,v);return url.href;
}
export async function installFixture(context, options={}) {
 const traffic=[], unexpected=[], state={rows:[row(),row(ref('sira','102')),row(ref('mora',null,'99'))],generation,...options};
 await context.route('**/api/v1/**',async route=>{
  const request=route.request(),url=new URL(request.url()),path=url.pathname;
  traffic.push({method:request.method(),url:url.href,body:request.postData()});
  let value;const gen={generation:state.generation};
  if(request.method()!=='GET') {
   if(path==='/api/v1/attachments'&&state.allowUpload) value={id:'attachment-1',name:url.searchParams.get('name'),media_type:url.searchParams.get('media_type'),size:4,url:'/api/v1/attachments/attachment-1?generation='+generation};
   else {unexpected.push({method:request.method(),path});return route.fulfill({status:409,json:{error:{message:'Verification forbids this operation'}}});}
  }else if(path==='/api/v1/management') {if(state.descriptorGate)await state.descriptorGate;value={...gen,scope,project:'/fixture',tools:[],profiles:[],max_page:2,max_text:65536};}
  else if(path==='/api/v1/agents')value={...gen,agents:state.paged&&!url.searchParams.has('after')?state.rows.filter(r=>r.agent.name!=='mora'):state.rows,next:state.paged&&!url.searchParams.has('after')?'page-2':null,current_complete:!state.paged||url.searchParams.has('after')};
  else if(path==='/api/v1/workspaces')value={...gen,workspaces:[{path:'/fixture',sources:['explicit-membership']},{path:'/empty',sources:['role-marker']}],coverage:{errors:[],agent_limit_reached:false,scan_limit_reached:false}};
  else if(path==='/api/v1/office-activity')value={...gen,next_after:'baseline',has_more:false,messages:[],created:[],human_messages:[],human_has_more:false,human_unhandled_count:1};
  else if(path==='/api/v1/human-mailbox')value={...gen,messages:[{id:url.searchParams.get('before')?'old-letter':'letter',at:'2026-11-16T12:00:00Z',from:'mora',sender:ref(),text:url.searchParams.get('before')?'Older simulated letter':'Latest simulated letter',handled_at:null}],next_before:url.searchParams.get('before')?null:'cursor-old',unhandled_count:1};
  else if(path==='/api/v1/roles')value={...gen,workspace:'/fixture',roles:[]};
  else if(path.endsWith('/launch-relations'))value={...gen,origin:'unknown',upstream:null,downstream:[],next:null};
  else if(path.endsWith('/messages'))value={...gen,messages:[],next_after:'end',has_more:false};
  else if(/^\/api\/v1\/agents\/[^/]+$/.test(path))value=state.rows.find(r=>r.agent.name===decodeURIComponent(path.split('/').at(-1))&&r.agent.incarnation===url.searchParams.get('incarnation')&&r.agent.stopped_event===url.searchParams.get('stopped_event'));
  if(!value&&/^\/api\/v1\/agents\/[^/]+$/.test(path))return route.fulfill({status:404,json:{error:{message:'Exact fixture target unavailable'}}});
  if(!value){unexpected.push({method:request.method(),path});return route.fulfill({status:404,json:{error:{message:'Unsupported fixture path '+path}}});}
  return route.fulfill({json:value});
 });return {traffic,unexpected,state};
}
