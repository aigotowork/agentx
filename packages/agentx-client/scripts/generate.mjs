import { createClient } from '@hey-api/openapi-ts';
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const [mode, candidate, selection, ...rest] = process.argv.slice(2);
let sources;
if (mode === '--cloud') {
  if (!candidate || (selection && !['admin', 'namespace'].includes(selection)) || rest.length) {
    throw new Error('Usage: npm run generate -- --cloud CANDIDATE [admin|namespace]');
  }
  sources = (selection ? [selection] : ['admin', 'namespace']).map(surface => ({
    domain: `cloud-${surface}`,
    read: () => execFileSync(resolve(candidate), ['cloud-openapi', surface], { encoding: 'utf8' }),
  }));
} else {
  if (candidate) throw new Error('Usage: npm run generate -- [NATIVE_OPENAPI_DIRECTORY]');
  const directory = resolve(mode ?? `${root}/../../.docs/api-panorama/openapi`);
  sources = ['management', 'workbench', 'observation'].map(domain => {
    const input = `${directory}/${domain}.json`;
    return { domain, input, read: () => readFile(input, 'utf8') };
  });
}
const documents = await Promise.all(sources.map(async ({ domain, input, read }) => {
  const document = JSON.parse(await read());
  if (!document.openapi || !document.paths || !document.components?.schemas) throw new Error(`Expected actual emitted ${domain} OpenAPI`);
  return { domain, input, document };
}));
for (const { domain, input, document } of documents) {
  const output = `${root}/src/generated/${domain}`;
  await rm(output, { recursive: true, force: true });
  // The generator projects readOnly/writeOnly variants by mutating object inputs.
  // Keep the native document pristine for the packaged spec and wire codec.
  await createClient({ input: input ?? structuredClone(document), output: { path: output, postProcess: [] }, plugins: [
    { name: '@hey-api/typescript', $resolvers: { number: ctx => ['u-int64', 'uint64', 'int64'].includes(ctx.schema.format) ? ctx.$.type('bigint') : undefined } },
    '@hey-api/client-fetch', '@hey-api/sdk',
  ] });
  await mkdir(output, { recursive: true });
  await writeFile(`${output}/wire-schema.gen.ts`, `// Generated from actual emitted OpenAPI. Do not edit.\nexport const wireDocument = ${JSON.stringify(document, null, 2)} as const;\n`);
  const types = await readFile(`${output}/types.gen.ts`, 'utf8');
  const exported = new Set([...types.matchAll(/^export type (\w+) =/gm)].map(match => match[1]));
  const aliases = Object.entries(document.components.schemas).map(([name, schema]) => {
    const candidates = [name, name.replace(/(^|[^a-zA-Z0-9])([a-zA-Z0-9])/g, (_, _separator, c) => c.toUpperCase())];
    const writable = Object.values(schema.properties ?? {}).some(field => field.writeOnly === true);
    const generated = [...(writable ? candidates.map(candidate => `${candidate}Writable`) : []), ...candidates]
      .find(candidate => exported.has(candidate));
    if (!generated) throw new Error(`Cannot associate actual schema ${name} with generated type; fix naming configuration`);
    return `  ${JSON.stringify(name)}: Types.${generated};`;
  });
  await writeFile(`${output}/schema-types.gen.ts`, `// Generated from actual emitted components. Do not edit.\nimport type * as Types from './types.gen.js';\nexport type SchemaTypes = {\n${aliases.join('\n')}\n};\n`);
  await writeFile(`${root}/${domain}.openapi.json`, `${JSON.stringify(document, null, 2)}\n`);
}
