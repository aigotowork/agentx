/** Bundle the public browser SDK into the existing finite native asset table. */
import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';
export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)), base: '/', publicDir: false,
  build: { emptyOutDir: false, sourcemap: false, assetsInlineLimit: 0,
    rollupOptions: { output: { hashCharacters: 'hex', entryFileNames: 'assets-[hash].js', chunkFileNames: 'chunk-[hash].js', assetFileNames: 'assets-[hash][extname]' } } },
});
