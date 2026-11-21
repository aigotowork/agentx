import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import {mountSpaceInspection} from './inspection';
import "../uiux/systems/space/styles/index.css";
import "./viewport.css";
import "./scene/studio.css";

const env = import.meta.env;
const embeddedScope = document.querySelector('meta[name="agentx-space-scope"]')?.getAttribute('content');
const trustedHost = embeddedScope != null
  ? { baseUrl: location.origin, expectedScope: embeddedScope, embedded: true }
  : env.VITE_AGENT_SPACE_MANAGEMENT_URL !== undefined || env.VITE_AGENT_SPACE_EXPECTED_SCOPE !== undefined
  ? { baseUrl: env.VITE_AGENT_SPACE_MANAGEMENT_URL, expectedScope: env.VITE_AGENT_SPACE_EXPECTED_SCOPE, pageLimit: env.VITE_AGENT_SPACE_PAGE_LIMIT }
  : undefined;

const inspection = mountSpaceInspection();
createRoot(document.getElementById("root")!).render(<StrictMode><App inspection={inspection} host={trustedHost} previewCaption={env.VITE_AGENT_SPACE_PREVIEW_CAPTION} /></StrictMode>);
if (import.meta.hot) import.meta.hot.dispose(() => inspection.dispose());
