/** @aiPurpose Own original tool vector artwork for the shared playground and browser pointers. */
export const toolArt = {
  hammer:
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" shape-rendering="crispEdges"><path d="M10 8h4v2h2v2h2v2h2v2h2v2h2v4h-4v-2h-2v-2h-2v-2h-2v-2h-2v-2h-2V8Z" fill="#19254B"/><path d="M12 10h2v2h2v2h2v2h2v2h2v2h-2v-2h-2v-2h-2v-2h-2v-2h-2v-2Z" fill="#FFC938"/><path d="M20 18h2v2h-2v-2Z" fill="#C94B0A"/><path d="M2 8h2V6h4V4h4V2h4v2h2v4h-2v2h-4v2H8v2H4v-2H2V8Z" fill="#19254B"/><path d="M4 8h4V6h4V4h4v4h-4v2H8v2H4V8Z" fill="#315BFF"/><path d="M4 8h4V6h4v2H8v2H4V8Z" fill="#FFFFFF"/><path d="M12 4h4v2h-4V4Z" fill="#7096FF"/></svg>',
  shovel:
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" shape-rendering="crispEdges"><path d="M2 2h10v10h-2v2H8v6h4v2H2v-2h4v-6H4v-2H2V2Z" fill="#19254B"/><path d="M4 4h6v6H8v2H6v-2H4V4Z" fill="#008563"/><path d="M4 4h2v6H4V4Z" fill="#43E4AD"/><path d="M6 14h2v6H6v-6Z" fill="#FFC938"/><path d="M4 20h6v2H4v-2Z" fill="#C94B0A"/><path d="M14 10h4v2h2v2h-6v-4Z" fill="#D9E0F0"/></svg>',
  chisel:
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" shape-rendering="crispEdges"><path d="M2 2h8v4h2v2h2v2h2v2h2v2h2v6h-6v-2h-2v-2h-2v-2H8v-2H6v-2H4V8H2V2Z" fill="#19254B"/><path d="M4 4h4v2H4V4Z" fill="#FFFFFF"/><path d="M6 6h4v2h2v2h2v2h2v2h2v4h-2v-2h-2v-2h-2v-2h-2v-2H8V8H6V6Z" fill="#713CE0"/><path d="M6 6h2v2H6V6Zm2 2h2v2H8V8Zm2 2h2v2h-2v-2Z" fill="#BB95FF"/><path d="M16 16h2v2h-2v-2Z" fill="#FFC938"/></svg>',
} as const;
export type ToolArtName = keyof typeof toolArt;
/** @aiPurpose Select vector resources by semantic tool role. @aiUsage Use as img src or CSS cursor; no network asset is fetched. */
export function toolImage(name: ToolArtName): string {
  return `data:image/svg+xml,${encodeURIComponent(toolArt[name])}`;
}
