// Run from the built package or an external project with @agentx/client installed.
import { randomUUID } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { OutcomeUnknown, ReceiptError, selectAgentXScope } from '@agentx/client';

const [mode, baseUrl, expectedScope, commandPath, addressPath, text, ...extra] = process.argv.slice(2);
if (!baseUrl || !expectedScope || !commandPath || extra.length ||
    !(mode === 'send' && addressPath && text?.trim() || mode === 'observe' && addressPath === undefined)) {
  console.error('Usage: node scoped-send.mjs send BASE_URL EXPECTED_SCOPE COMMAND_FILE AGENT_ADDRESS_FILE TEXT');
  console.error('       node scoped-send.mjs observe BASE_URL EXPECTED_SCOPE COMMAND_FILE');
  process.exit(2);
}

const scope = await selectAgentXScope({ baseUrl, expectedScope });
if (mode === 'observe') {
  const saved = JSON.parse(await readFile(commandPath, 'utf8'));
  console.log(await scope.operation(saved.operation).wait());
} else {
  // Save snapshot.address from the exact Agent selected in an earlier read.
  const agent = scope.agent(JSON.parse(await readFile(addressPath, 'utf8')));
  if (agent.kind !== 'current') throw new Error('Sending requires a current exact Agent address');
  const prepared = agent.prepareMessage({ text, intent: 'inform' }, randomUUID());
  await writeFile(commandPath, JSON.stringify(prepared), { flag: 'wx', mode: 0o600, flush: true });
  let receipt;
  try {
    receipt = await agent.send(prepared);
  } catch (error) {
    if (!(error instanceof OutcomeUnknown || error instanceof ReceiptError)) throw error;
    console.error('Submission receipt unavailable or inconsistent; observing the original operation.');
  }
  if (!receipt || receipt.status === 'Accepted' || receipt.status === 'Running') receipt = await scope.operation(prepared.operation).wait();
  console.log(receipt);
}
