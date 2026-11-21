import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";
const apiTarget = process.env.AGENT_SPACE_API_TARGET;
export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)), base: process.env.AGENT_SPACE_BASE ?? "./",
  server: { host: process.env.AGENT_SPACE_HOST ?? "127.0.0.1", port: Number(process.env.AGENT_SPACE_PORT ?? 18967), strictPort: true, hmr: false, watch: {usePolling: true, interval: 1000}, proxy: apiTarget ? { "/api": { target: apiTarget, changeOrigin: true, configure(proxy) { proxy.on("proxyReq", request => { request.setHeader("origin", apiTarget); request.setHeader("sec-fetch-site", "same-origin"); }); } } } : undefined },
  build: { assetsDir: "assets", sourcemap: false },
});
