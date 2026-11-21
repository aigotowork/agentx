import { openDialog } from './disclosure.js';

const people = [
  { key: 'mira-7f6a4e2c', name: 'Mira', role: 'Release coordination', state: 'Working', tone: 'success', activity: 'Mapped the staging checklist to the two pending verification steps.' },
  { key: 'sol-48c9a5d1', name: 'Sol', role: 'Review', state: 'Waiting', tone: 'info', activity: 'Waiting for the receipt before reviewing the rollout note.' },
  { key: 'emi-5c1ea8f0', name: 'Emi', role: 'Build support', state: 'Unavailable', tone: 'neutral', activity: 'Last known activity: recorded a build summary. Current activity is unavailable.' },
];

const $ = id => document.getElementById(id);
const participants = $('participants');
const workspace = $('workspace');
const search = $('search');
const draft = $('draft');
const review = $('review');
const noteDialog = $('note-dialog');
const receiptDialog = $('receipt-dialog');
const drafts = new Map();
const saved = new Map();
let subject = people[0];
let receiptKnown = false;
let noteSnapshot;

function setDraft() {
  draft.value = drafts.get(subject.key) ?? '';
  updateDraftControls();
}

function updateDraftControls() {
  $('draft-length').textContent = `${draft.value.length} chars`;
  review.disabled = !draft.value.trim();
}

function renderList() {
  const query = search.value.toLowerCase().trim();
  const matched = people.filter(person => `${person.name} ${person.role}`.toLowerCase().includes(query));
  participants.replaceChildren(...matched.map(person => {
    const row = document.createElement('li');
    row.className = 'ax-row';
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'ax-button';
    button.dataset.key = person.key;
    button.setAttribute('aria-pressed', String(subject.key === person.key));
    const identity = document.createElement('strong');
    identity.textContent = person.name;
    const role = document.createElement('span');
    role.className = 'meta';
    role.textContent = person.role;
    identity.append(role);
    const badge = document.createElement('span');
    badge.className = 'ax-badge';
    badge.dataset.tone = person.tone;
    badge.textContent = person.state;
    button.append(identity, badge);
    button.addEventListener('click', () => select(person));
    row.append(button);
    return row;
  }));
  $('empty').hidden = matched.length !== 0;
}

function renderSubject() {
  $('subject-title').textContent = subject.name;
  $('subject-role').textContent = subject.role;
  $('subject-state').textContent = subject.state;
  $('subject-state').dataset.tone = subject.tone;
  $('identity').textContent = subject.key;
  $('record-author').textContent = subject.name;
  $('record').textContent = saved.has(subject.key) ? `Local note: ${saved.get(subject.key)}` : subject.activity;
  $('notice').hidden = subject !== people[0];
  $('receipt').hidden = subject !== people[0];
  for (const button of participants.querySelectorAll('button')) {
    button.setAttribute('aria-pressed', String(button.dataset.key === subject.key));
  }
  setDraft();
}

function renderReceipt() {
  const state = receiptKnown ? 'Receipt confirmed' : 'Outcome unknown';
  const tone = receiptKnown ? 'success' : 'warning';
  $('notice-title').textContent = state;
  $('notice-detail').textContent = receiptKnown ? 'The synthetic receipt was located.' : 'The same request needs receipt lookup.';
  $('notice').dataset.tone = tone;
  $('receipt-state').textContent = state;
  $('receipt-state').dataset.tone = tone;
  $('lookup').textContent = receiptKnown ? 'View receipt' : 'Look up';
}

function select(person) {
  const activeFacts = $('exact-facts');
  if (activeFacts.matches(':popover-open')) activeFacts.hidePopover();
  subject = person;
  workspace.dataset.view = 'detail';
  renderSubject();
  if ($('back').getClientRects().length > 0) $('subject-title').focus();
}

function announce(message) {
  $('live-feedback').textContent = message;
}

$('back').addEventListener('click', () => {
  workspace.dataset.view = 'index';
  (participants.querySelector('[aria-pressed="true"]') ?? search).focus();
});
search.addEventListener('input', renderList);
draft.addEventListener('input', () => {
  drafts.set(subject.key, draft.value);
  updateDraftControls();
});
$('style').addEventListener('change', event => { document.documentElement.dataset.axStyle = event.target.value; });
$('brightness').addEventListener('change', event => { document.documentElement.dataset.axBrightness = event.target.value; });
$('receipt').addEventListener('click', event => openDialog(receiptDialog, event.currentTarget));
$('lookup').addEventListener('click', event => {
  if (receiptKnown) openDialog(receiptDialog, event.currentTarget);
  else {
    receiptKnown = true;
    renderReceipt();
    announce('Synthetic receipt confirmed.');
  }
});
review.addEventListener('click', () => {
  noteSnapshot = { key: subject.key, text: draft.value };
  $('note-target').textContent = `${subject.name} · ${subject.key}`;
  $('note-text').textContent = draft.value;
  openDialog(noteDialog, review);
});
noteDialog.addEventListener('close', () => {
  if (noteDialog.returnValue !== 'save' || !noteSnapshot) return;
  saved.set(noteSnapshot.key, noteSnapshot.text);
  if (drafts.get(noteSnapshot.key) === noteSnapshot.text) drafts.delete(noteSnapshot.key);
  renderSubject();
  if (review.disabled && (document.activeElement === review || document.activeElement === document.body)) draft.focus();
  announce('Local note saved.');
});
$('reset').addEventListener('click', () => {
  drafts.clear();
  saved.clear();
  receiptKnown = false;
  subject = people[0];
  workspace.dataset.view = 'index';
  search.value = '';
  renderList();
  renderSubject();
  renderReceipt();
});

renderList();
renderSubject();
renderReceipt();
