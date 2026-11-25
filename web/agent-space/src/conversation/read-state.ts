/** Adapter-independent state for latest-first conversation and Transcript views. */
export type ReadState = { phase: 'loading' | 'ready' | 'error' | 'unavailable'; error?: string };
export type HistoryReadPage = ReadState & {
  more: boolean;
  older?: ReadState;
  live?: ReadState;
  token?: object;
  arrivals?: number;
  emptyOlder?: boolean;
};
