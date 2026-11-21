import assert from 'node:assert/strict';

const kinds = new Set(['interface', 'class', 'value', 'union', 'enum', 'method', 'outcome', 'capability', 'external', 'utility']);
const relations = new Set(['owns', 'accepts', 'returns', 'contains', 'references', 'observes', 'rejects', 'composes', 'variant', 'supports', 'calls', 'aliases', 'extends', 'implements', 'wraps', 'delegates']);
const authorities = new Set(['semantic', 'declaration', 'implementation', 'generated', 'native', 'platform', 'proposal']);
const provenance = new Set(['source-backed', 'proposed']);
const visibility = new Set(['public', 'implementation']);
const text = (value, where) => assert(typeof value === 'string' && value.length > 0, where);
const array = (value, where) => assert(Array.isArray(value), where);
const unique = (values, where) => {
  assert.equal(new Set(values).size, values.length, `duplicate ${where}`);
};

/** Validate portable contract facts and return exact anchor indexes for view/project checks. */
export function validateModel(model) {
  assert.equal(model.schemaVersion, 1);
  for (const key of ['id', 'title', 'authority']) text(model[key], key);
  for (const key of ['layers', 'nodes', 'edges', 'sources', 'scenarios']) {
    array(model[key], key);
    for (const item of model[key]) {
      assert(item && typeof item === 'object' && !Array.isArray(item), `${key}: expected record`);
      text(item.id, `${key} ID`);
    }
  }
  assert(model.layers.length > 0, 'at least one layer is required');
  const nodes = new Map(model.nodes.map(n => [n.id, n]));
  const layers = new Map(model.layers.map(layer => [layer.id, layer]));
  const sources = new Map(model.sources.map(s => [s.id, s]));
  unique(model.nodes.map(n => n.id), 'node IDs');
  unique(model.sources.map(s => s.id), 'source IDs');
  unique(model.edges.map(e => e.id), 'edge IDs');
  unique(model.scenarios.map(s => s.id), 'scenario IDs');
  unique(model.layers.map(layer => layer.id), 'layer IDs');
  const anchors = new Map(nodes);
  const fieldOwners = new Map();
  for (const node of model.nodes) {
    array(node.fields, `${node.id}.fields`);
    for (const field of node.fields) {
      text(field.name, `${node.id} field name`);
      assert(!anchors.has(field.id), `duplicate field/node anchor ${field.id}`);
      assert.equal(field.id, `${node.id}.${field.name}`);
      anchors.set(field.id, field);
      fieldOwners.set(field.id, node.id);
    }
  }
  const refs = (ids, where, target = nodes) => {
    array(ids, where);
    unique(ids, where);
    for (const id of ids) assert(target.has(id), `${where}: unresolved ${id}`);
  };
  const evidence = (ids, where) => refs(ids, `${where}.sourceRefs`, sources);
  const typed = (value, where) => {
    assert(value && typeof value === 'object', `${where}: expected typed value`);
    text(value.type, `${where}.type`);
    refs(value.typeIds, `${where}.typeIds`);
    for (const key of ['optional', 'nullable']) {
      if (key in value) assert.equal(typeof value[key], 'boolean', `${where}.${key}`);
    }
  };
  const ownerNode = id => nodes.get(id) || nodes.get(fieldOwners.get(id));
  for (const layer of model.layers) {
    for (const key of ['id', 'name', 'summary']) text(layer[key], `layer.${key}`);
    assert(provenance.has(layer.status), `${layer.id}.status`);
    evidence(layer.sourceRefs, layer.id);
    assert(model.nodes.some(node => node.layerId === layer.id && node.kind !== 'method' && node.visibility !== 'implementation'), `${layer.id}: needs a public type`);
  }
  for (const source of model.sources) {
    for (const key of ['id', 'path', 'symbol']) text(source[key], `source ${key}`);
    assert(authorities.has(source.authority), source.id);
    assert(Number.isInteger(source.line) && source.line > 0, source.id);
  }
  for (const node of model.nodes) {
    for (const key of ['id', 'name', 'domain', 'summary']) text(node[key], `${node.id}.${key}`);
    assert(kinds.has(node.kind), `${node.id}.kind`);
    assert(layers.has(node.layerId), `${node.id}.layerId`);
    assert(provenance.has(node.provenance), `${node.id}.provenance`);
    if ('visibility' in node) assert(visibility.has(node.visibility), `${node.id}.visibility`);
    if ('signature' in node) text(node.signature, `${node.id}.signature`);
    if ('support' in node) {
      assert(node.support && typeof node.support === 'object' && !Array.isArray(node.support), `${node.id}.support`);
      for (const [key, value] of Object.entries(node.support)) text(value, `${node.id}.support.${key}`);
    }
    evidence(node.sourceRefs, node.id);
    refs(node.ports, `${node.id}.ports`);
    for (const field of node.fields) {
      text(field.summary, `${field.id}.summary`);
      typed(field, field.id);
      assert(!('layerId' in field) && !('provenance' in field), `${field.id}: inherits declaration ownership`);
      if ('visibility' in field) assert(visibility.has(field.visibility), `${field.id}.visibility`);
    }
    // The declarative fact boundary must never acquire renderer geometry or a hidden scene owner.
    for (const key of ['x', 'y', 'width', 'height', 'layout', 'scene', 'camera', 'route']) {
      assert(!(key in node), `${node.id}: layout leaked into facts (${key})`);
    }
    for (const port of node.ports) {
      assert.equal(nodes.get(port).kind, 'method');
      assert.equal(nodes.get(port).owner, node.id);
    }
    if (node.kind === 'method') {
      assert(nodes.has(node.owner) && nodes.get(node.owner).kind !== 'method', `${node.id}.owner`);
      assert(nodes.get(node.owner).ports.includes(node.id), `${node.id}: owner missing port`);
      assert.equal(node.layerId, nodes.get(node.owner).layerId, `${node.id}: port layer differs from owner`);
      assert.equal(node.provenance, nodes.get(node.owner).provenance, `${node.id}: port provenance differs from owner`);
      assert(node.fields.length === 0 && node.ports.length === 0, `${node.id}: method cannot own fields or ports`);
      array(node.inputs, `${node.id}.inputs`);
      unique(node.inputs.map(i => i.name), `${node.id} parameter names`);
      for (const input of node.inputs) { text(input.name, node.id); typed(input, node.id); }
      typed(node.output, `${node.id}.output`);
      refs(node.errors, `${node.id}.errors`);
      refs(node.outcomes, `${node.id}.outcomes`);
      assert(['local', 'read', 'submit', 'observe'].includes(node.effect), `${node.id}.effect`);
    } else {
      for (const key of ['owner', 'inputs', 'output', 'errors', 'outcomes', 'effect']) {
        assert(!(key in node), `${node.id}.${key}: only method nodes may declare this`);
      }
    }
  }
  const hasEdge = (from, to, kind) => model.edges.some(e => e.from === from && e.to === to && (!kind || e.kind === kind));
  const declarationOwners = (id, visited = new Set()) => {
    if (visited.has(id)) return visited;
    visited.add(id);
    for (const edge of model.edges.filter(edge => edge.from === id && edge.kind === 'extends')) declarationOwners(edge.to, visited);
    return visited;
  };
  const canReceive = (id, target, visited = new Set()) => {
    if (visited.has(id)) return false;
    if (declarationOwners(id).has(target) || hasEdge(id, target, 'implements')) return true;
    if (nodes.get(id)?.kind !== 'union') return false;
    const variants = model.edges.filter(edge => edge.from === id && edge.kind === 'variant' && nodes.has(edge.to));
    return variants.length > 0 && variants.every(edge => canReceive(edge.to, target, new Set([...visited, id])));
  };
  for (const edge of model.edges) {
    assert(anchors.has(edge.from), `${edge.id}: missing source anchor ${edge.from}`);
    assert(anchors.has(edge.to), `${edge.id}: missing target anchor ${edge.to}`);
    assert(relations.has(edge.kind), edge.id);
    assert.equal(edge.id, `${edge.from}|${edge.kind}|${edge.to}`);
    text(edge.label, `${edge.id}.label`);
    evidence(edge.sourceRefs, edge.id);
    assert(provenance.has(edge.provenance), `${edge.id}.provenance`);
    const from = ownerNode(edge.from), to = ownerNode(edge.to);
    if (from.provenance === 'proposed' || to.provenance === 'proposed') assert.equal(edge.provenance, 'proposed', `${edge.id}: proposed endpoint`);
    if (edge.provenance === 'source-backed' && edge.sourceRefs.length) assert(edge.sourceRefs.some(id => sources.get(id).authority !== 'proposal'), `${edge.id}: source relationship cannot cite only proposal evidence`);
    if (edge.kind === 'extends') {
      assert(nodes.has(edge.from) && nodes.has(edge.to), `${edge.id}: extension requires declarations`);
      assert(['class', 'interface'].includes(from.kind) && from.kind === to.kind, `${edge.id}: only class-class or interface-interface extension`);
    }
    if (edge.kind === 'implements') assert(nodes.has(edge.from) && nodes.has(edge.to) && from.kind === 'class' && to.kind === 'interface', `${edge.id}: class implements interface`);
    if (edge.kind === 'wraps') {
      assert(fieldOwners.has(edge.from) && nodes.has(edge.to), `${edge.id}: wrapping must identify receiver field and held value`);
      assert(anchors.get(edge.from).typeIds.includes(edge.to), `${edge.id}: held value absent from field type`);
    }
    if (edge.kind === 'delegates') {
      assert(from.kind === 'method' && to.kind === 'method', `${edge.id}: delegation requires method ports`);
      assert(fieldOwners.has(edge.via), `${edge.id}: missing receiver field`);
      assert(declarationOwners(from.owner).has(fieldOwners.get(edge.via)), `${edge.id}: receiver belongs to method owner or its base`);
      const held = model.edges.filter(candidate => candidate.from === edge.via && candidate.kind === 'wraps');
      assert(held.length === 1 && canReceive(held[0].to, to.owner), `${edge.id}: receiver cannot supply target port`);
      text(edge.mapping, `${edge.id}.mapping`);
      assert.equal(from.effect, to.effect, `${edge.id}: effect mismatch requires a separately modeled orchestration`);
      assert(to.errors.every(id => from.errors.includes(id)), `${edge.id}: hidden callee failures`);
      assert(to.outcomes.every(id => from.outcomes.includes(id)), `${edge.id}: hidden callee result branches`);
      if (to.sourceRefs.length) assert(edge.sourceRefs.some(id => to.sourceRefs.includes(id)), `${edge.id}: needs target declaration evidence`);
    }
    if (edge.kind === 'implements') {
      for (const portId of to.ports) {
        const required = nodes.get(portId);
        const supplied = [...declarationOwners(from.id)].flatMap(id => nodes.get(id).ports).map(id => nodes.get(id)).find(port => port.name === required.name);
        assert(supplied, `${edge.id}: missing interface method ${required.name}`);
        assert.deepEqual(supplied.inputs, required.inputs, `${edge.id}: input correspondence`);
        assert.deepEqual(supplied.output, required.output, `${edge.id}: output correspondence`);
      }
      for (const field of to.fields) {
        const supplied = [...declarationOwners(from.id)].flatMap(id => nodes.get(id).fields).find(candidate => candidate.name === field.name);
        assert(supplied && supplied.type === field.type, `${edge.id}: missing/incompatible interface field ${field.name}`);
      }
    }
  }
  // Declaration extension is acyclic; union membership and composition are deliberately separate.
  const bases = id => model.edges.filter(edge => edge.from === id && edge.kind === 'extends').map(edge => edge.to);
  const checkBases = (id, path = new Set()) => {
    assert(!path.has(id), `${id}: cyclic declaration extension`);
    for (const base of bases(id)) checkBases(base, new Set([...path, id]));
  };
  for (const node of model.nodes) checkBases(node.id);
  // Every declared field and method dependency must have a corresponding directed edge.
  for (const node of model.nodes) {
    for (const field of node.fields) {
      assert(hasEdge(node.id, field.id, 'owns'), `${field.id}: missing field ownership edge`);
      for (const id of field.typeIds) assert(hasEdge(field.id, id), `${field.id}: missing typed dependency ${id}`);
    }
    if (node.kind !== 'method') continue;
    assert(hasEdge(node.owner, node.id, 'owns'), `${node.id}: missing port ownership edge`);
    for (const input of node.inputs) for (const id of input.typeIds) assert(hasEdge(node.id, id, 'accepts'), `${node.id}: missing input ${id}`);
    for (const id of node.output.typeIds) assert(hasEdge(node.id, id, 'returns'), `${node.id}: missing output ${id}`);
    for (const id of node.errors) assert(hasEdge(node.id, id, 'rejects'), `${node.id}: missing rejection ${id}`);
    for (const id of node.outcomes) assert(hasEdge(node.id, id), `${node.id}: missing outcome ${id}`);
  }
  for (const scenario of model.scenarios) {
    for (const key of ['id', 'name', 'summary']) text(scenario[key], key);
    evidence(scenario.sourceRefs, scenario.id);
    assert(provenance.has(scenario.provenance), `${scenario.id}.provenance`);
    array(scenario.steps, `${scenario.id}.steps`);
    assert(scenario.steps.length > 0, `${scenario.id}: needs a checkpoint`);
    for (const step of scenario.steps) {
      text(step.label, scenario.id); refs(step.nodeIds, scenario.id, anchors);
      assert(step.nodeIds.length > 0, `${scenario.id}: checkpoint needs a subject`);
    }
    if (scenario.steps.some(step => step.nodeIds.some(id => ownerNode(id)?.provenance === 'proposed'))) assert.equal(scenario.provenance, 'proposed', `${scenario.id}: proposed scenario must stay labeled`);
  }
  return {nodes, layers, anchors, sources, ownerNode, refs, hasEdge};
}

/** Validate scene bindings without inferring workflow from project names or ID spelling. */
export function validateView(model, view, {nodes, layers, refs, ownerNode}) {
  const record = (value, where) => assert(value && typeof value === 'object' && !Array.isArray(value), where);
  const strings = (value, keys, where) => {
    record(value, where);
    for (const key of keys) if (key in value) text(value[key], `${where}.${key}`);
  };
  const publicType = (id, where, layerId) => {
    const node = nodes.get(id);
    assert(node && node.kind !== 'method' && node.visibility !== 'implementation' && (!layerId || node.layerId === layerId), `${where}: expected public type in the selected layer (${id})`);
  };
  const method = (id, where) => assert(nodes.get(id)?.kind === 'method', `${where}: expected method (${id})`);
  record(view, 'view');
  assert.equal(view.schemaVersion, 1, 'view.schemaVersion');
  if ('brand' in view) strings(view.brand, ['name', 'title', 'ariaLabel', 'edition', 'editionNumber', 'footer'], 'view.brand');
  if ('initial' in view) {
    record(view.initial, 'initial');
    assert(layers.has(view.initial.layerId), 'initial.layerId: unresolved layer');
    if ('subjectId' in view.initial) publicType(view.initial.subjectId, 'initial.subjectId', view.initial.layerId);
  }
  if ('layers' in view) record(view.layers, 'view.layers');
  for (const [id, entry] of Object.entries(view.layers ?? {})) {
    assert(layers.has(id), `view.layers: unresolved ${id}`);
    strings(entry, ['label'], `view.layers.${id}`);
    if ('initialSubject' in entry) publicType(entry.initialSubject, 'layer.initialSubject', id);
    if ('order' in entry) {
      refs(entry.order, `view.layers.${id}.order`);
      for (const nodeId of entry.order) publicType(nodeId, 'layer.order', id);
    }
  }
  if ('domains' in view) record(view.domains, 'view.domains');
  const domainIds = new Set(model.nodes.map(n => n.domain));
  for (const [id, entry] of Object.entries(view.domains ?? {})) {
    assert(domainIds.has(id), `view.domains: unresolved ${id}`);
    strings(entry, ['label'], `view.domains.${id}`);
    if ('color' in entry) assert(['blue', 'teal', 'violet', 'amber'].includes(entry.color), `view.domains.${id}.color`);
  }
  if ('supportLabels' in view) {
    record(view.supportLabels, 'supportLabels');
    for (const label of Object.values(view.supportLabels)) text(label, 'supportLabels label');
  }
  if ('proposedSupportKeys' in view) {
    array(view.proposedSupportKeys, 'proposedSupportKeys');
    for (const key of view.proposedSupportKeys) text(key, 'proposedSupportKeys key');
  }
  if ('neighborOrder' in view) refs(view.neighborOrder, 'neighborOrder');
  if ('overview' in view) {
    const o = view.overview;
    strings(o, ['tab', 'title'], 'overview');
    publicType(o.subjectId, 'overview.subjectId');
    const layer = ownerNode(o.subjectId).layerId;
    array(o.groups, 'overview.groups');
    assert(o.groups.length > 0, 'overview.groups: needs a group');
    const members = new Set();
    for (const group of o.groups) {
      text(group.label, 'overview group label');
      refs(group.nodeIds, 'overview group nodes');
      assert(group.nodeIds.length >= 1 && group.nodeIds.length <= 4, 'overview group requires 1–4 types');
      for (const id of group.nodeIds) {
        publicType(id, 'overview group', layer);
        assert(!members.has(id), `duplicate overview type ${id}`);
        members.add(id);
      }
    }
    if ('ports' in o) {
      refs(o.ports, 'overview.ports');
      const owners = new Set();
      for (const id of o.ports) {
        method(id, 'overview.ports');
        const owner = nodes.get(id).owner;
        assert(members.has(owner) && !owners.has(owner), `overview.ports: expected one port per displayed owner (${id})`);
        owners.add(owner);
      }
    }
    if ('highlightFrom' in o) refs([o.highlightFrom], 'overview.highlightFrom');
  }
  if ('methodAnnotations' in view) record(view.methodAnnotations, 'methodAnnotations');
  for (const [id, targets] of Object.entries(view.methodAnnotations ?? {})) {
    method(id, 'methodAnnotations');
    refs(targets, `methodAnnotations.${id}`);
    for (const target of targets) assert(nodes.get(target).kind !== 'method', `method annotation expects a type (${target})`);
  }
  if ('walkthrough' in view) {
    const w = view.walkthrough;
    strings(w, ['tab', 'title', 'kicker'], 'walkthrough');
    const scenario = model.scenarios.find(s => s.id === w.scenarioId);
    assert(scenario, 'walkthrough.scenarioId: unresolved scenario');
    assert(layers.has(w.layerId), 'walkthrough.layerId: unresolved layer');
    for (const step of scenario.steps) assert(step.nodeIds.some(id => ownerNode(id).layerId === w.layerId), 'walkthrough: every checkpoint needs a subject in its layer');
    if ('stepTitles' in w) {
      array(w.stepTitles, 'walkthrough.stepTitles');
      assert.equal(w.stepTitles.length, scenario.steps.length, 'walkthrough.stepTitles: one title per checkpoint');
      for (const title of w.stepTitles) text(title, 'checkpoint title');
    }
  }
  if ('sequence' in view) {
    const s = view.sequence;
    strings(s, ['tab', 'title', 'kicker', 'stepHeading', 'branchHeading', 'stepLabel', 'branchLabel'], 'sequence');
    array(s.lanes, 'sequence.lanes');
    assert(s.lanes.length >= 2 && s.lanes.length <= 3, 'sequence: needs two or three lanes');
    for (const lane of s.lanes) {
      text(lane.label, 'sequence lane label');
      strings(lane, ['narrowLabel'], 'sequence lane');
    }
    const laneIndex = i => Number.isInteger(i) && i >= 0 && i < s.lanes.length;
    array(s.steps, 'sequence.steps');
    assert(s.steps.length > 0, 'sequence: needs a step');
    for (const step of s.steps) {
      refs([step.id], 'sequence step subject');
      text(step.label, 'sequence step label');
      text(step.caption, 'sequence step caption');
      assert(laneIndex(step.from) && laneIndex(step.to) && step.from !== step.to, 'sequence step: invalid lane pair');
      if ('error' in step) assert.equal(typeof step.error, 'boolean', 'sequence step error flag');
    }
    if ('branchSource' in s) {
      refs([s.branchSource], 'sequence.branchSource');
      const branches = model.edges.filter(e => e.from === s.branchSource && e.kind === 'variant');
      assert(branches.length >= 1 && branches.length <= 6 && branches.every(e => nodes.has(e.to)), 'sequence: expected 1–6 variant nodes in the sequence layer');
    }
    if ('annotation' in s) {
      strings(s.annotation, ['label', 'narrowLabel', 'ariaLabel', 'caption', 'narrowCaption'], 'sequence.annotation');
      refs([s.annotation.id], 'sequence.annotation');
    }
  }
  if ('lenses' in view) {
    array(view.lenses, 'lenses');
    for (const lens of view.lenses) {
      text(lens.id, 'lens id');
      text(lens.label, 'lens label');
      strings(lens, ['detail'], 'lens');
      if (lens.scene === 'method') method(lens.subjectId, 'lens.subjectId');
      else assert(lens.scene === 'sequence' && view.sequence, 'lens requires a method or declared sequence');
    }
    unique(view.lenses.map(l => l.id), 'lens IDs');
  }
}
