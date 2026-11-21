/* Pure layer projection over authored facts; included by the static builder. */
(() => {
  'use strict';
  const model = globalThis.CONTRACT_MODEL;
  const nodes = new Map(model.nodes.map(node => [node.id, node]));
  const owners = new Map(model.nodes.flatMap(node => node.fields.map(field => [field.id, node.id])));
  const nodeFor = id => nodes.get(id) || nodes.get(owners.get(id));
  const layers = model.layers.map(layer => ({ ...layer, title: layer.name, nodeIds: model.nodes.filter(node => node.layerId === layer.id).map(node => node.id) }));
  const byId = new Map(layers.map(layer => [layer.id, layer]));
  globalThis.CONTRACT_LAYERS = Object.freeze({
    layers,
    layer: id => byId.get(id),
    layerForNode: id => byId.get(nodeFor(id)?.layerId),
    layerMappings: id => !byId.has(id) ? [] : model.edges.filter(edge => {
      const from = nodeFor(edge.from)?.layerId, to = nodeFor(edge.to)?.layerId;
      return from !== to && (from === id || to === id);
    }),
  });
})();
