import { createCloudAdminClient, createCloudScopeClient } from '@agentx/client';

const client = createCloudAdminClient({ baseUrl: process.env.AGENTX_ADMIN_URL, token: process.env.AGENTX_ADMIN_TOKEN });
const [overview, page, detail, hosts, agents] = await Promise.all([
  client.overview(), client.listScopes({ limit: 1 }), client.getScope('cli-managed'),
  client.listHosts('cli-managed'), client.listAgents('cli-managed'),
]);
const scope = createCloudScopeClient({ baseUrl: detail.scope_url, token: process.env.AGENTX_SCOPE_TOKEN });
const descriptor = await scope.describe();
const station = { generation: descriptor.generation, installation_id: crypto.randomUUID() };
const host = await scope.renewHost('sdk-host', station);
const agent = await scope.registerAgent('sdk-agent', { ...station, host: host.name, incarnation: crypto.randomUUID() });
const scoped = await scope.listAgents({ host: host.name });
const cross = await client.listAgents(detail.id);
console.log(JSON.stringify({ overview, page, detail, hosts, agents, descriptor, host, agent, scoped, cross },
  (_, value) => typeof value === 'bigint' ? value.toString() : value));
