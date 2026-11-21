/** Decorative addressed message flights; native history and delivery cursors remain external facts. */
import { HUMAN_ENDPOINT, type SceneMessage } from "../../offline/model";
import type { MessageFlight, SceneInput } from "./contracts";

export class MessageEffects {
  private seen: Set<string>;
  private flights: MessageFlight[] = [];
  private geometry = "";

  constructor(input: SceneInput) { this.seen = new Set(input.messages?.map(item => item.id)); }

  sample(input: SceneInput, elapsed: number, wallNow: number, still: boolean): readonly MessageFlight[] {
    const geometry = JSON.stringify(input.scene.actors.map(actor => [actor.key, actor.home, actor.rest]));
    if (this.geometry && this.geometry !== geometry) this.flights = [];
    this.geometry = geometry;
    for (const event of input.messages ?? []) {
      if (this.seen.has(event.id)) continue;
      this.seen.add(event.id);
      if (!still && wallNow - event.receivedAt < 4000 && this.flights.length < 24) {
        this.flights.push({ event, started: elapsed + Math.floor(this.flights.length / 6) * 700 });
      }
    }
    if (still) this.flights = [];
    const keys = new Set([HUMAN_ENDPOINT, ...input.scene.actors.map(actor => actor.key)]);
    this.flights = this.flights.filter(flight => elapsed - flight.started < 700 && keys.has(flight.event.from) && keys.has(flight.event.to));
    if (this.seen.size > 256) this.seen = new Set((input.messages ?? []).map(item => item.id));
    return this.flights.slice(0, 6);
  }

  restore(messages: readonly SceneMessage[]): void {
    this.flights = [];
    for (const item of messages) this.seen.add(item.id);
  }

  dispose(): void { this.flights = []; this.seen.clear(); this.geometry = ""; }
}
