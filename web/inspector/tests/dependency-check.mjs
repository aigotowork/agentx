/** Inspect actual production inputs separately from browser network evidence. */
import {readFile,writeFile,readdir} from 'node:fs/promises';
import assert from 'node:assert/strict';
const root=new URL('../',import.meta.url);
const pkg=JSON.parse(await readFile(new URL('package.json',root),'utf8'));
const lock=JSON.parse(await readFile(new URL('package-lock.json',root),'utf8'));
const meta=JSON.parse(await readFile(new URL('dist/metafile.json',root),'utf8'));
const forbidden=/(?:@getpaseo|@paseo|paseo-client|paseo-sdk|bots-sdk)/i;
const packages=Object.keys(lock.packages).filter(x=>forbidden.test(x));
const inputs=Object.keys(meta.inputs),badInputs=inputs.filter(x=>forbidden.test(x)||/plugin\/server\//.test(x));
const outside=inputs.filter(x=>x.startsWith('../')||x.startsWith('/'));
const imports=[];
for(const input of inputs.filter(x=>!x.includes('node_modules/'))){const s=await readFile(new URL(input,root),'utf8');for(const m of s.matchAll(/(?:from\s*|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g))if(forbidden.test(m[1]))imports.push({input,import:m[1]});}
const core=inputs.filter(x=>x.startsWith('plugin/core/'));
const report={manifestDependencies:pkg.dependencies,lockPackageCount:Object.keys(lock.packages).length,builtInputCount:inputs.length,forbiddenPackages:packages,forbiddenBuiltInputs:badInputs,outsideModuleInputs:outside,forbiddenImports:imports,portableCoreInputs:core,scope:'Manifest, lockfile, complete esbuild metafile, and executable non-node_modules production input imports. Historical license/resource prose excluded from dependency claims.'};
await writeFile(new URL('tests/evidence/2026-11-15/dependency-result.json',root),JSON.stringify(report,null,2));
assert.deepEqual(packages,[]);assert.deepEqual(badInputs,[]);assert.deepEqual(outside,[]);assert.deepEqual(imports,[]);for(const family of ['stats-facts','stats-snapshot','tokens/admission','tokens/snapshot','digest/fold','recovery','backoff'])assert(core.some(x=>x.includes(family)),`Portable core absent from built inputs: ${family}`);
console.log(JSON.stringify({status:'passed',builtInputCount:inputs.length,portableCoreFiles:core.length}));
