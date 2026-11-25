import { createCloudAdminClient, createCloudNamespaceClient, newCloudId } from '@agentx/client';

const client = createCloudAdminClient({ baseUrl: process.env.AGENTX_ADMIN_URL, token: process.env.AGENTX_ADMIN_TOKEN });
const [overview, page, detail, hosts, agents] = await Promise.all([
  client.overview(), client.listNamespaces({ limit: 1 }), client.getNamespace('cli-managed'),
  client.listHosts('cli-managed'), client.listAgents('cli-managed'),
]);
const scope = createCloudNamespaceClient({ baseUrl: detail.namespace_url, token: process.env.AGENTX_NAMESPACE_TOKEN });
const descriptor = await scope.describe();
const station = { generation: descriptor.generation, installation_id: crypto.randomUUID() };
const host = await scope.renewHost('sdk-host', station);
const agent = await scope.registerAgent('sdk-agent', { ...station, host: host.name, incarnation: newCloudId(),
  workspace: { kind: 'unexposed' } });
const scoped = await scope.listAgents({ generation: descriptor.generation, host: host.name });
const cross = await client.listAgents(detail.namespace_id, { generation: detail.generation });
const workspaces = await scope.listWorkspaces({ generation: descriptor.generation, host: host.name, limit: 1 });
const adminWorkspaces = await client.listWorkspaces(detail.namespace_id, { generation: detail.generation, host: host.name, limit: 1 });
console.log(JSON.stringify({ overview, page, detail, hosts, agents, descriptor, host, agent, scoped, cross, workspaces, adminWorkspaces },
  (_, value) => typeof value === 'bigint' ? value.toString() : value));
