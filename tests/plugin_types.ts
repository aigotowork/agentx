// Type-only check: Kilo loads the same plugin as OpenCode 1, so the plugin must
// fit both apps' own `Plugin` types (hook shapes and the client calls it makes).
import type { Plugin as OpenCodePlugin } from "@opencode-ai/plugin"
import type { Plugin as KiloPlugin } from "@kilocode/plugin"
import { AgentxPlugin } from "../src/integrations/plugins/opencode/agentx.ts"

export const openCode: OpenCodePlugin = AgentxPlugin
export const kilo: KiloPlugin = AgentxPlugin
