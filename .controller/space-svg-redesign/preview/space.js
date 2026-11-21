import { people, rooms } from './fixtures.js';

const $ = (id) => document.getElementById(id);
const world = $('world');
const stage = world.closest('.stage');
const draft = $('draft');
const letter = $('letter');
const transcript = $('transcript');
const state = {
  room: 'workshop', selected: null, reading: null, index: false,
  drafts: new Map(), added: new Map(), pending: new Set(),
};
const svg = (name, attrs = {}) => {
  const node = document.createElementNS('http://www.w3.org/2000/svg', name);
  Object.entries(attrs).forEach(([key, value]) => node.setAttribute(key, value));
  return node;
};
const icon = (id) => '<svg class="icon" aria-hidden="true"><use href="#i-' + id + '"/></svg>';
const figure = (id) => './art/cast.svg#' + id;

function fitScene() {
  const width = stage.clientWidth;
  if (!width) return;
  const span = width < 520 ? 470 : width < 975 ? Math.max(470, width * 1.015) : 1000;
  const height = span === 1000 ? 590 : Math.min(590, Math.max(465, span * stage.clientHeight / width));
  world.setAttribute('viewBox', [(1000 - span) / 2 + (span < 525 ? 20 : 0), (590 - height) / 2, span, height].join(' '));
}

function drawAtmosphere() {
  if (state.room === 'workshop') {
    const pulse = svg('g', { class: 'pulse', fill: '#f3c979' });
    pulse.append(svg('ellipse', { cx: 383, cy: 188, rx: 9, ry: 9, opacity: .27 }),
      svg('circle', { cx: 383, cy: 188, r: 3 }));
    $('room-art').append(pulse);
  } else {
    const eddy = svg('path', { d: 'M623 340q7-7 1-12t2-12', class: 'eddy', fill: 'none',
      stroke: '#f7d6aa', 'stroke-width': '2', 'stroke-linecap': 'round' });
    $('room-art').append(eddy);
  }
}

function drawRoom() {
  const room = rooms[state.room];
  stage.dataset.room = state.room;
  stage.style.setProperty('--room-tint', room.tint);
  $('room-title').textContent = room.title;
  const occupants = Object.entries(people).filter(([, person]) => person.room === state.room);
  $('population').textContent = occupants.length + ' 人在场';
  Object.keys(rooms).forEach((id) => $(id).setAttribute('aria-pressed', id === state.room));
  $('room-art').replaceChildren(svg('use', { href: './art/rooms.svg#' + state.room }));
  drawAtmosphere();
  $('cast').replaceChildren();
  $('tags').replaceChildren();

  for (const [id, person] of occupants) {
    const actor = svg('g', { id: 'actor-' + id, class: 'actor-control', transform: 'translate(' + person.x + ' ' + person.y + ')', 'aria-hidden': 'true' });
    const performer = svg('g', { class: 'performer ' + person.motion });
    const body = svg('g', { class: 'body-gait' });
    performer.append(svg('ellipse', { class: 'actor-ground', cx: 0, cy: 1, rx: 39, ry: 11 }),
      svg('ellipse', { class: 'actor-ring', cx: 0, cy: 0, rx: 43, ry: 13 }));
    if (id === 'rowan') {
      body.append(svg('use', { class: 'boot boot-left', href: figure('rowan-left') }),
        svg('use', { class: 'boot boot-right', href: figure('rowan-right') }),
        svg('use', { href: figure('rowan-upper') }));
    } else {
      body.append(svg('use', { href: figure(id) }));
    }
    body.append(svg('rect', { x: -42, y: -120, width: 84, height: 123, fill: 'transparent', 'pointer-events': 'all' }));
    performer.append(body);
    if (id === 'qi') {
      performer.append(svg('path', { d: 'M1-57q6-6 1-11t2-11', class: 'eddy', fill: 'none',
        stroke: '#ead1ad', 'stroke-width': '2', 'stroke-linecap': 'round' }));
    }
    actor.append(performer);
    actor.addEventListener('click', () => openLetter(id));
    $('cast').append(actor);

    const tag = svg('foreignObject', { x: person.x - 82, y: person.y + 8, width: 164, height: 69 });
    const button = document.createElement('button');
    button.className = 'actor-tag';
    button.type = 'button';
    button.id = 'talk-' + id;
    button.dataset.uiComponent = 'ActorTalkTag';
    button.setAttribute('aria-controls', 'letter');
    button.setAttribute('aria-label', '与' + person.name + '对话 · ' + person.activity);
    button.style.setProperty('--actor-plate', person.plate);
    button.innerHTML = '<svg class="tag-paper" aria-hidden="true"><use href="#plaque" width="100%" height="100%"/></svg><span class="tag-id"><strong>' +
      person.name + '</strong><small>' + icon(person.glyph) + person.activity + '</small></span>' + icon('talk');
    button.addEventListener('click', () => openLetter(id));
    tag.append(button);
    $('tags').append(tag);
  }
  if (state.room === 'workshop') drawHandoff();
  syncSelection();
  fitScene();
}

// The handoff shares the workshop cast's CSS clock. It has no separate action or hit target.
function drawHandoff() {
  const trail = svg('g', { class: 'courier-thread', 'aria-hidden': 'true', 'pointer-events': 'none' });
  const route = 'M514 321C563 321 620 338 658 384';
  trail.append(svg('path', { d: route, class: 'thread-shadow' }),
    svg('path', { d: route, class: 'thread-line' }),
    svg('path', { d: 'm514 317 4 4-4 4-4-4Z m144 63 4 4-4 4-4-4Z', class: 'thread-knots' }));
  $('room-art').append(trail);
  const position = svg('g', { transform: 'translate(464 252)', 'aria-hidden': 'true', 'pointer-events': 'none' });
  const handoff = svg('g', { class: 'handoff', stroke: '#543724', 'stroke-width': '1.9', 'stroke-linejoin': 'round' });
  handoff.append(svg('path', { d: 'M-14-7 13-7 13 9-14 9Z', fill: '#efce8f' }),
    svg('path', { d: 'm-14-7 14 10 13-10m-27 16 11-8m16 8L3 1', fill: 'none' }),
    svg('path', { d: 'M18-8l5-4m-3 10 6-1m-45-3-4-4', fill: 'none', stroke: '#f5d094' }));
  position.append(handoff);
  $('cast').append(position);
}

function syncSelection() {
  for (const id of Object.keys(people)) {
    $('actor-' + id)?.classList.toggle('is-selected', state.selected === id);
    $('actor-' + id)?.classList.toggle('is-reading', state.reading === id);
    $('talk-' + id)?.classList.toggle('is-selected', state.selected === id);
    $('talk-' + id)?.setAttribute('aria-expanded', state.reading === id);
  }
}

function drawPortrait(target, id) {
  const person = people[id];
  target.replaceChildren(
    svg('path', { d: 'M4 16 16 4h51l13 12v51L67 80H16L4 67Z', fill: person.plate, stroke: '#a3875e', 'stroke-width': '2' }),
    svg('path', { d: 'M10 20 21 9h43l10 11v44L64 74H21L10 64Z', fill: 'none', stroke: person.color, 'stroke-width': '1' }),
    svg('path', { d: 'M42 13 71 42 42 71 13 42Z', fill: person.color, opacity: '.38' }),
  );
  const crop = svg('svg', { x: 12, y: 10, width: 60, height: 62, viewBox: '-43 -123 86 82' });
  crop.append(svg('use', { href: figure(id) }));
  target.append(crop);
}

function drawIndex() {
  const count = Object.keys(people).length;
  $('index-count').textContent = String(count).padStart(2, '0');
  $('roster-count').textContent = count;
  for (const [id, person] of Object.entries(people)) {
    const item = document.createElement('li');
    const button = document.createElement('button');
    button.className = 'person-row';
    button.type = 'button';
    button.innerHTML = '<svg class="portrait" aria-hidden="true" viewBox="0 0 84 84"></svg><span><strong>' +
      person.name + '</strong><small>' + rooms[person.room].title + ' · ' + person.activity +
      '</small></span>' + icon('talk');
    drawPortrait(button.firstElementChild, id);
    button.addEventListener('click', () => openLetter(id));
    item.append(button);
    $('people-list').append(item);
  }
}

function showIndex(open, restore = false) {
  state.index = open;
  $('people').setAttribute('aria-expanded', open);
  $('people-index').hidden = !open;
  if (open) $('people-list').querySelector('button').focus();
  else if (restore) $('people').focus();
}

function addEntry(id, [speaker, timestamp, line], pending = false) {
  const row = document.createElement('div');
  row.className = 'entry ' + speaker + (pending ? ' pending' : '');
  const meta = document.createElement('div');
  meta.className = 'entry-meta';
  meta.textContent = speaker === 'you' ? '你' : people[id].name;
  const time = document.createElement('time');
  time.textContent = timestamp;
  meta.append(time);
  const content = document.createElement('p');
  content.textContent = line;
  if (pending) content.setAttribute('aria-label', people[id].name + '正在输入');
  row.append(meta, content);
  transcript.append(row);
  return row;
}

function drawConversation(id) {
  const rule = document.createElement('div');
  rule.className = 'entry-rule';
  rule.textContent = '今晚 · 夜班';
  transcript.replaceChildren(rule);
  for (const entry of [...people[id].history, ...(state.added.get(id) ?? [])]) addEntry(id, entry);
  if (state.pending.has(id)) addEntry(id, ['them', '', '···'], true);
  transcript.scrollTop = transcript.scrollHeight;
}

function openLetter(id) {
  const person = people[id];
  state.selected = id;
  state.reading = id;
  showIndex(false);
  if (state.room !== person.room) {
    state.room = person.room;
    drawRoom();
  } else {
    syncSelection();
  }
  $('workspace').classList.add('has-letter');
  $('letter-name').textContent = person.name;
  $('letter-room').textContent = rooms[person.room].title;
  $('letter-role').textContent = person.role + ' · ' + person.activity;
  drawPortrait($('portrait'), id);
  draft.value = state.drafts.get(id) ?? '';
  draft.placeholder = '给' + person.name + '写消息…';
  letter.hidden = false;
  drawConversation(id);
  updateSend();
  $('letter-name').focus({ preventScroll: true });
}

function closeLetter(restore = true) {
  const previous = state.reading;
  state.reading = null;
  letter.hidden = true;
  $('workspace').classList.remove('has-letter');
  syncSelection();
  if (restore && previous) $('talk-' + previous)?.focus({ preventScroll: true });
}

function updateSend() {
  $('send').disabled = !draft.value.trim() || state.pending.has(state.reading);
}

function append(id, entry) {
  if (!state.added.has(id)) state.added.set(id, []);
  state.added.get(id).push(entry);
}

for (const id of Object.keys(rooms)) {
  $(id).addEventListener('click', () => {
    showIndex(false);
    if (id === state.room) return;
    closeLetter(false);
    state.room = id;
    state.selected = null;
    drawRoom();
  });
}
$('people').addEventListener('click', () => showIndex(!state.index));
$('close-people').addEventListener('click', () => showIndex(false, true));
$('close-letter').addEventListener('click', () => closeLetter());
document.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape' || event.isComposing) return;
  if (state.index) showIndex(false, true);
  else if (state.reading) closeLetter();
});
draft.addEventListener('input', () => {
  state.drafts.set(state.reading, draft.value);
  updateSend();
});
draft.form.addEventListener('submit', (event) => {
  event.preventDefault();
  const id = state.reading;
  const value = draft.value.trim();
  if (!id || !value || state.pending.has(id)) return;
  const time = '19:' + (42 + Math.floor((state.added.get(id)?.length ?? 0) / 2) % 18);
  const first = !state.added.get(id)?.length;
  append(id, ['you', time, value]);
  state.drafts.delete(id);
  state.pending.add(id);
  draft.value = '';
  addEntry(id, ['you', time, value]);
  addEntry(id, ['them', '', '···'], true);
  transcript.scrollTop = transcript.scrollHeight;
  updateSend();
  draft.focus({ preventScroll: true });
  window.setTimeout(() => {
    state.pending.delete(id);
    append(id, ['them', time, first ? people[id].reply : '收到，已经补进同一条记录。']);
    if (state.reading === id) {
      transcript.querySelector('.pending')?.remove();
      addEntry(id, state.added.get(id).at(-1));
      transcript.scrollTop = transcript.scrollHeight;
      updateSend();
    }
  }, 780);
});
drawIndex();
drawRoom();
new ResizeObserver(fitScene).observe(stage);
