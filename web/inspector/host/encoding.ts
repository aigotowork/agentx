/** Small browser adapters preserve source SHA-256 keys and UTF-8 payload budgets. */
import { sha256 } from '@noble/hashes/sha2.js';
import { bytesToHex } from '@noble/hashes/utils.js';
export const byteLength = (value:string,_encoding?:string) => new TextEncoder().encode(value).byteLength;
export function createHash(algorithm:'sha256') {if(algorithm!=='sha256')throw new Error('Unsupported digest');return {update(value:string){return {digest(_encoding:'hex'){return bytesToHex(sha256(new TextEncoder().encode(value)));}};}};}
