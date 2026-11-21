/** Parent harness; only hello and addContext are emitted. Native send stays in the child composer. */
const field = id => document.getElementById(id);
const frame = field('conversation'), status = field('status'), add = field('add');
const protocol = 'agentx.context.v1';
let origin, target, helloTimer;
function hello() { frame.contentWindow.postMessage({ protocol, type: 'hello', requestId: 'parent-ready' }, origin); }
field('open').addEventListener('submit', event => {
  event.preventDefault(); clearInterval(helloTimer); add.disabled = true;
  try {
    target = JSON.parse(field('target').value);
    const url = new URL('/space/', field('service').value); origin = url.origin;
    url.search = new URLSearchParams({ scope: target.scope, generation: target.generation, agent: target.agent.name, ...(target.agent.incarnation !== null ? { incarnation: target.agent.incarnation } : { stopped_event: target.agent.stopped_event }), view: 'chat', layout: 'sidebar' });
    frame.src = url.href; status.value = 'Waiting for iframe…';
  } catch (error) { status.value = error.message; }
});
frame.addEventListener('load', () => { if (origin) { hello(); clearInterval(helloTimer); helloTimer = setInterval(hello, 750); } });
window.addEventListener('message', event => {
  if (event.source !== frame.contentWindow || event.origin !== origin || event.data?.protocol !== protocol) return;
  if (event.data.type === 'ready') { clearInterval(helloTimer); add.disabled = false; status.value = 'Bridge ready. Context will enter a reviewable draft.'; }
  else if (event.data.type === 'result') status.value = JSON.stringify(event.data, null, 2);
});
field('context').addEventListener('submit', event => {
  event.preventDefault();
  // If bytes arrive through an extension runtime JSON hop, reconstruct File/Blob here first.
  frame.contentWindow.postMessage({ protocol, type: 'addContext', requestId: crypto.randomUUID(), target, context: { title: field('title').value, text: field('text').value, url: field('source').value, files: Array.from(field('files').files) } }, origin);
  status.value = 'Adding context…';
});
