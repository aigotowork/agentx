/** @aiPurpose Render lazy plugin surfaces with local loading, retry and language-change feedback. */
import { createElement, useEffect, useState, type ComponentType, type ReactNode } from "react";
import { ActivityIndicator, Pressable, Text, View } from "react-native";
import type { PluginClientContext } from "../../host/adapter";
import { onTextChange, t, translator } from "../shared/i18n";
import { createChunkLoader, type ChunkLoaderOptions } from "./chunk-loader";

export { createChunkLoader, type ChunkExtras } from "./chunk-loader";

interface Options extends ChunkLoaderOptions {
  fallback?: (id: string, props: any) => ReactNode;
}

/** Which text a view was drawn with; a change of language or catalog redraws lazy views from scratch. */
function useTextVersion(): string {
  const [, setChanges] = useState(0);
  useEffect(() => onTextChange(() => setChanges((value) => value + 1)), []);
  const current = translator();
  return `${current.language}:${current.hash}`;
}

/**
 * @aiPurpose Adapt the loader to ordinary screen, panel and timeline components.
 * @aiUsage Create once per client registration; no Suspense needed.
 */
/** @aiPurpose Provide lazySurfaces presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function lazySurfaces(client: PluginClientContext, options: Options): (id: string) => ComponentType<any> {
  const loader = createChunkLoader(client, options);
  return (id) => function LazySurface(props: any) {
    const [state, setState] = useState<{ Component?: ComponentType<any>; error?: string }>(() => ({ Component: loader.peek()?.[id] }));
    const [attempt, setAttempt] = useState(0);
    const version = useTextVersion();
    useEffect(() => {
      if (state.Component) return;
      let live = true;
      loader.load().then((ready) => {
        if (live) setState(ready[id] ? { Component: ready[id] } : { error: t("kitLazy.versionMismatch") });
      }, (error: unknown) => { if (live) setState({ error: error instanceof Error ? error.message : String(error) }); });
      return () => { live = false; };
    }, [attempt]);
    if (state.Component) return createElement(state.Component, { ...props, key: version });
    const colors = props.theme?.colors ?? {};
    return <View style={{ flex: 1, alignItems: "center", justifyContent: "center", gap: 10, padding: 16 }}>
      {state.error ? <><Text style={{ color: colors.foreground }}>{t("kitLazy.failed", { label: loader.label(), error: state.error })}</Text>
        <Pressable accessibilityRole="button" onPress={() => { setState({}); setAttempt((value) => value + 1); }}><Text style={{ color: colors.accent }}>{t("kitLazy.retry")}</Text></Pressable></>
        : options.fallback ? options.fallback(id, props) : <><ActivityIndicator color={colors.foregroundMuted} /><Text style={{ color: colors.foregroundMuted }}>{t("kitLazy.loading", { label: loader.label() })}</Text></>}
    </View>;
  };
}
