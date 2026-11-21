/** @aiPurpose Build the fixed-scope management with external same-origin assets. */
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

/**
 * @aiPurpose Produce flat hashed same-origin assets for the native finite asset table.
 * @aiUsage build.mjs supplies a fresh Cargo-owned output directory.
 */
export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)),
  plugins: [react()],
  base: "/",
  publicDir: false,
  server: { host: "127.0.0.1", port: 18749, strictPort: true },
  build: {
    emptyOutDir: false,
    sourcemap: false,
    assetsInlineLimit: 0,
    rollupOptions: { output: { hashCharacters: "hex", entryFileNames: "assets-[hash].js", chunkFileNames: "chunk-[hash].js", assetFileNames: "assets-[hash][extname]" } },
  },
});
