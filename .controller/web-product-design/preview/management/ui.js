export const e = value => String(value??'').replace(/[&<>"']/g,x=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[x]));
const paths={
  logo:'<rect x="3" y="3" width="18" height="18" rx="5"/><path d="m7 16 5-9 5 9M9.5 13h5"/>',
  agents:'<rect x="3" y="5" width="18" height="14" rx="3"/><path d="M8 10v1m8-1v1m-7 4h6M12 2v3"/>',
  workspaces:'<path d="M3 7h7l2 2h9v10H3zM3 7V5h7l2 2h7v2"/>',
  operations:'<path d="M6 3h12v18l-3-2-3 2-3-2-3 2zM9 8h6m-6 4h6m-6 4h3"/>',
  search:'<circle cx="10.5" cy="10.5" r="5.5"/><path d="m15 15 5 5"/>',
  plus:'<path d="M12 5v14M5 12h14"/>',
  arrow:'<path d="M5 12h14l-5-5m5 5-5 5"/>',
  chevron:'<path d="m9 6 6 6-6 6"/>',
  up:'<path d="M8 16 17 7M8 7h9v9"/>',
  collapse:'<path d="m6 9 6 6 6-6"/>',
  refresh:'<path d="M20 11a8 8 0 1 0-2 6M20 5v6h-6"/>',
  more:'<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
  info:'<circle cx="12" cy="12" r="9"/><path d="M12 11v5m0-8v.1"/>',
  branch:'<circle cx="6" cy="5" r="2"/><circle cx="18" cy="6" r="2"/><circle cx="6" cy="19" r="2"/><path d="M6 7v10m0-3h5a7 7 0 0 0 7-6"/>',
  envelope:'<rect x="3" y="5" width="18" height="14" rx="3"/><path d="m4 7 8 6 8-6"/>',
  clock:'<circle cx="12" cy="12" r="8"/><path d="M12 7v5l3 2"/>',
  close:'<path d="m6 6 12 12M18 6 6 18"/>',
  shield:'<path d="M12 3 4 6v6c0 4 4 7 8 9 4-2 8-5 8-9V6zm-3 9 2 2 4-4"/>',
  type:'<path d="M4 6h16m-8 0v13m-4 0h8"/>',
  list:'<path d="M4 6h16M4 12h16M4 18h16"/>',
  send:'<path d="m3 11 17-7-6 17-3-7zm8 3 9-10"/>'
};
export const icon = name => '<svg aria-hidden="true" class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">'+(paths[name]||paths.info)+'</svg>';
export const badge = (label,tone='neutral')=>'<span class="ax-badge state-badge" data-tone="'+tone+'"><span class="state-dot" aria-hidden="true"></span>'+e(label)+'</span>';
export const btn = (label,cmd,{glyph,variant='quiet',disabled=false,value='',id=''}={})=>'<button type="button" class="ax-button" data-variant="'+variant+'" data-cmd="'+cmd+'" data-value="'+e(value)+'"'+(disabled?' disabled':'')+(id?' id="'+id+'"':'')+'>'+(glyph?icon(glyph):'')+e(label)+'</button>';
export const ibtn=(label,cmd,glyph)=>'<button type="button" class="ax-button ax-icon-button" data-variant="quiet" data-cmd="'+cmd+'" aria-label="'+e(label)+'" title="'+e(label)+'">'+icon(glyph)+'</button>';
export const option = (value,label,selected)=>'<option value="'+e(value)+'"'+(value===selected?' selected':'')+'>'+e(label)+'</option>';
export const notice = (msg,tone='warning')=>'<div class="ax-notice" data-tone="'+tone+'">'+icon('info')+'<span>'+e(msg)+'</span></div>';
export const field = (title,content,wide=false)=>'<label class="form-field'+(wide?' field-wide':'')+'"><span>'+title+'</span>'+content+'</label>';
export const input = (name,value='',attrs='')=>'<input class="ax-input" name="'+name+'" value="'+e(value)+'" '+attrs+'>';
export const select = (name,options)=>'<select class="ax-input" name="'+name+'">'+options+'</select>';
export const empty = (title,detail,glyph='list')=>'<div class="empty-state">'+icon(glyph)+'<strong>'+e(title)+'</strong><span>'+e(detail)+'</span></div>';
export const refTitle = target => {
  if(!target) return '';
  const [name, ref, id]=target.split('~');
  return id?'@'+name+' · '+(ref==='incarnation'?'执行':'停止事件')+' #'+id:target;
};
