// Run from the built package or an external project with @agentx/client installed.
import { randomUUID } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { OutcomeUnknown, ReceiptError, selectAgentXScope } from '@agentx/client';

const [mode, baseUrl, expectedScope, commandPath, workspaceKey, name, text] = process.argv.slice(2);
if (!baseUrl || !expectedScope || !commandPath || !['send', 'observe'].includes(mode) || (mode === 'send' && (!workspaceKey || !name || !text))) {
  console.error('Usage: node scoped-send.mjs send BASE_URL EXPECTED_SCOPE COMMAND_FILE WORKSPACE NAME TEXT');
  console.error('       node scoped-send.mjs observe BASE_URL EXPECTED_SCOPE COMMAND_FILE');
  process.exit(2);
}

const scope = await selectAgentXScope({ baseUrl, expectedScope });
if (mode === 'observe') {
  const saved = JSON.parse(await readFile(commandPath, 'utf8'));
  console.log(await scope.operation(saved.operation).wait());
} else {
  const catalog = await scope.workspaces();
  if (!catalog.workspaces.some(row => row.workspace.address.key === workspaceKey)) {
    console.error('Workspace was not discovered; using the explicit key and native Role read.', catalog.coverage);
  }
  const roles = await scope.workspace(workspaceKey).roles();
  console.error('Effective Roles:', roles.roles.map(role => role.definition.name));
  // This example examines one bounded page. A caller can choose to continue
  // with coverage.next or use an exact address retained from an earlier read.
  const page = await roles.workspace.members({ limit: scope.descriptor.max_page });
  const member = page.members.find(({ agent }) => agent.kind === 'current' && agent.address.reference.name === name);
  if (!member || member.agent.kind !== 'current') {
    console.error('Target was not observed on this page:', page.coverage);
    process.exitCode = 1;
  } else {
    const { agent } = member;
    const snapshot = await agent.snapshot();
    const history = await agent.messages({ limit: 10 });
    console.error('Current status:', snapshot.facts.status, 'Messages in this page:', history.messages.length);
    const prepared = agent.prepareMessage({ text, intent: 'inform' }, randomUUID());
    await writeFile(commandPath, JSON.stringify(prepared), { flag: 'wx', mode: 0o600, flush: true });
    try {
      await agent.send(prepared);
    } catch (error) {
      if (!(error instanceof OutcomeUnknown || error instanceof ReceiptError)) throw error;
      console.error('Submission receipt unavailable or inconsistent; observing the original operation.');
    }
    console.log(await scope.operation(prepared.operation).wait());
  }
}
