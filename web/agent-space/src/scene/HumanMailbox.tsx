/** @aiPurpose Adapt native mailbox facts to the canonical UIUX Block using the established safe Markdown renderer. */
import { HumanMailboxBlock } from "../../uiux/systems/space/space.web";
import { Markdown } from "../conversation/Markdown";
import type { HumanFeed } from "../offline/model";
/** @aiPurpose Read supplied human correspondence without changing native handling or exact sender semantics.
 * @aiUsage Parent owns feed effects/navigation; this adapter supplies the existing inert-external-image Markdown renderer.
 */
export function HumanMailbox({ feed, onClose, onNavigate }: { feed?: HumanFeed; onClose(): void; onNavigate?(id: string): void }) {
  return <HumanMailboxBlock feed={feed} onClose={onClose} onNavigate={onNavigate} renderText={text => <Markdown text={text} />} />;
}
