import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve, extname, sep } from 'node:path';
const root = fileURLToPath(new URL('../dist/', import.meta.url));
const port = Number(process.env.INSPECTOR_PORT ?? 18971);
const mime = {'.html':'text/html; charset=utf-8','.js':'text/javascript; charset=utf-8','.css':'text/css; charset=utf-8','.svg':'image/svg+xml','.json':'application/json'};
createServer(async (req,res) => {
  const pathname = decodeURIComponent(new URL(req.url,'http://local').pathname);
  if (pathname.startsWith('/api/')) {res.writeHead(404);res.end('No live API: Inspector uses a local mock provider.');return;}
  const file = resolve(root, '.' + (pathname === '/' ? '/index.html' : pathname));
  if (!file.startsWith(root.endsWith(sep)?root:root+sep)) {res.writeHead(403);res.end();return;}
  try {const data=await readFile(file);res.writeHead(200,{'Content-Type':mime[extname(file)]??'application/octet-stream','Cache-Control':'no-store'});res.end(data);} catch {res.writeHead(404);res.end('Not found');}
}).listen(port,'0.0.0.0',()=>process.stdout.write(`AgentX Inspector mock: http://127.0.0.1:${port}\n`));
