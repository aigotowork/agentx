/** @aiPurpose Mount the parallel recovery Web with its presentation and crash boundaries. */
import { createRoot } from "react-dom/client";
import { Component, type ErrorInfo, type ReactNode } from "react";
import { OverlayBoundary } from "../../uiux/systems/recovery/recovery.web";
import "./app.css";
class ApplicationBoundary extends Component<
  { children: ReactNode },
  { error: boolean }
> {
  state = { error: false };
  static getDerivedStateFromError() {
    return { error: true };
  }
  componentDidCatch(_error: Error, _info: ErrorInfo) {}
  render() {
    return this.state.error ? (
      <main className="application-error">
        <h1>界面暂时不可用</h1>
        <p>原请求身份仍保存在本标签页。</p>
        <button className="button secondary" onClick={() => location.reload()}>
          重新载入
        </button>
      </main>
    ) : (
      this.props.children
    );
  }
}
const experience = new URLSearchParams(location.search).get('source') === 'demo' && new URLSearchParams(location.search).get('experience') === 'recovery' ? 'recovery' : 'inspector';
document.documentElement.dataset.experience = experience;
document.title = experience === 'recovery' ? '恢复调查 · Agent Inspector' : 'Agent Inspector';
if (experience === 'recovery') {
  const { RecoveryScreen } = await import('./recovery/screen');
  createRoot(document.getElementById("root")!).render(
  <ApplicationBoundary>
    <OverlayBoundary>
      <RecoveryScreen />
    </OverlayBoundary>
  </ApplicationBoundary>,
);
}
else {
  const { default: Inspector } = await import('./inspector');
  createRoot(document.getElementById('root')!).render(<ApplicationBoundary><Inspector /></ApplicationBoundary>);
}
