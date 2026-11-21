/** One pointer gesture: drift/cancel releases it; a fired hold consumes exactly one click. */
export class LongPress {
  private timer: ReturnType<typeof setTimeout> | undefined;
  private start?: { id: number; x: number; y: number };
  private fired = false;
  begin(id: number, x: number, y: number, activate: () => void): void {
    this.cancel(); this.fired = false; this.start = { id, x, y };
    this.timer = setTimeout(() => { this.timer = undefined; if (this.start) { this.fired = true; activate(); } }, 500);
  }
  move(id: number, x: number, y: number): void {
    if (this.start?.id === id && Math.hypot(x - this.start.x, y - this.start.y) > 10) this.cancel();
  }
  release(id: number): void { if (this.start?.id === id) { clearTimeout(this.timer); this.timer = undefined; this.start = undefined; } }
  cancel(): void { clearTimeout(this.timer); this.timer = undefined; this.start = undefined; }
  consumeClick(): boolean { const fired = this.fired; this.fired = false; return fired; }
}
