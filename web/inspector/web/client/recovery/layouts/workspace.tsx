/** @aiPurpose Arrange recovery regions and responsive scroll owners without domain decisions. */
import type { ReactNode } from "react";
/**
 * @aiPurpose Arrange queue and inspector regions with explicit responsive scroll owners.
 * @aiUsage Supply domain blocks as slots; the layout only owns arrangement.
 */
/** @aiPurpose Compose RecoveryLayout presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function RecoveryLayout({
  header,
  notice,
  queue,
  detail,
  footer,
  detailOpen,
}: {
  header: ReactNode;
  notice?: ReactNode;
  queue: ReactNode;
  detail: ReactNode;
  footer?: ReactNode;
  detailOpen: boolean;
}) {
  return (
    <div className="recovery-workspace">
      <header className="workspace-header">{header}</header>
      {notice}
      <main
        className={"workspace-columns " + (detailOpen ? "detail-open" : "")}
      >
        <div className="workspace-queue">{queue}</div>
        <div className="workspace-detail">{detail}</div>
      </main>
      {footer}
    </div>
  );
}
