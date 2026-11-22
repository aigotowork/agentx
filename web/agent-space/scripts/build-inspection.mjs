/** Build an isolated source-enabled preview using an explicitly supplied DancingAI checkout. */
import {resolve} from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {mkdir,cp} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {loadVite} from '../../vite.mjs';
const {build}=await loadVite();
const [checkout,target]=process.argv.slice(2);
if(!checkout||!target||!target.startsWith('/tmp/'))throw Error('Usage: node web/agent-space/scripts/build-inspection.mjs /absolute/dancingai-checkout /tmp/new-preview-directory');
const source=fileURLToPath(new URL('../',import.meta.url)),root=resolve(source,'../..');
const check=spawnSync(process.execPath,[resolve(root,'node_modules/typescript/bin/tsc'),'-p',resolve(source,'tsconfig.json'),'--noEmit'],{cwd:root,stdio:'inherit'});
if(check.error||check.status)process.exit(check.status||1);
const {objectSources}=await import(pathToFileURL(resolve(checkout,'packages/vite-plugin/dist/index.js')).href);
await mkdir(target,{recursive:false});
await build({configFile:resolve(source,'vite.config.ts'),base:'/space/',plugins:[objectSources({sourceRoot:root,reactComponents:true,includeInBuild:true})],build:{outDir:target,emptyOutDir:false,sourcemap:true}});
await cp(resolve(source,'NOTICE.md'),resolve(target,'NOTICE.md'));
