/** @aiPurpose Place compact office navigation beside the existing scene/chat workspace. */
import type { ReactNode, Ref } from "react";
/** @aiPurpose Own workspace sizing and nested scroll boundaries.
 * @aiUsage Pass office navigation and the active scene, roster or resource panel; no business state is owned here.
 */
export function SpaceLayout({ navigation, children, contentRef }: { navigation: ReactNode; children: ReactNode; contentRef?: Ref<HTMLDivElement> }) {
  return <div className="space-working-layout"><aside className="office-navigation">{navigation}</aside><div ref={contentRef} className="office-working-content">{children}</div></div>;
}
