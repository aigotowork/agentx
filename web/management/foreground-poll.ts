/** Continue routine refresh only while the page is visible. User actions run independently. */
export function foregroundPoll(run: () => Promise<void>, interval: number, signal: AbortSignal) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let running = false;
  const clear = () => clearTimeout(timer);
  const visible = () => !signal.aborted && !document.hidden;
  function tick() {
    clear();
    if (!visible() || running) return;
    running = true;
    void run().finally(() => {
      running = false;
      if (visible()) timer = setTimeout(tick, interval);
    });
  }
  document.addEventListener('visibilitychange', tick, { signal });
  signal.addEventListener('abort', clear, { once: true });
  tick();
}
