import { createRoot } from 'react-dom/client';
import './styles/management.css';
import './styles/interaction.css';

const root = createRoot(document.getElementById('root')!);
if (location.pathname === '/deployment' || location.pathname === '/deployment/') {
  document.title = 'AgentX · 云端部署';
  void import('./deployment/console').then(({ DeploymentConsole }) => root.render(<DeploymentConsole />));
} else {
  void import('./console').then(({ ManagementConsole }) => root.render(<ManagementConsole />));
}
