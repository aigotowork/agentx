// An exact address comes from a previously observed Agent snapshot or available relation.
import { randomUUID } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { MutationOutcomeUnknown, ReceiptError, selectAgentXScope } from '@agentx/client';

const [mode, baseUrl, expectedScope, commandFile, exactAddressFile] = process.argv.slice(2);
if (!baseUrl || !expectedScope || !commandFile || (mode !== 'observe' && (mode !== 'request' || !exactAddressFile))) {
  console.error('Usage: node scoped-transcript.mjs request BASE_URL EXPECTED_SCOPE COMMAND_FILE EXACT_AGENT_ADDRESS_FILE');
  console.error('       node scoped-transcript.mjs observe BASE_URL EXPECTED_SCOPE COMMAND_FILE');
  process.exit(2);
}

const scope = await selectAgentXScope({ baseUrl, expectedScope });
let prepared, receipt;
if (mode === 'request') {
  const address = JSON.parse(await readFile(exactAddressFile, 'utf8'));
  const agent = scope.agent(address);
  prepared = agent.prepareTranscript(randomUUID());
  await writeFile(commandFile, JSON.stringify(prepared), { flag: 'wx', mode: 0o600, flush: true });
  try {
    receipt = await agent.requestTranscript(prepared);
  } catch (error) {
    if (!(error instanceof MutationOutcomeUnknown || error instanceof ReceiptError)) throw error;
    console.error('No compatible submission receipt; observing the retained request without submitting again.');
  }
} else {
  prepared = JSON.parse(await readFile(commandFile, 'utf8'));
  if (prepared.kind !== 'transcript') throw new TypeError('Expected an original Transcript command');
}

if (!receipt || receipt.status === 'Accepted' || receipt.status === 'Running') receipt = await scope.operation(prepared).wait();
if (receipt.status !== 'Succeeded') {
  console.error(receipt.status, receipt.error);
  process.exitCode = 1;
} else {
  console.log(JSON.stringify(receipt.result, null, 2));
}
