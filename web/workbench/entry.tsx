/** @aiPurpose Mount the fixed-scope AgentX collaboration workbench without mock data. */
import { createRoot } from "react-dom/client";
import { WorkbenchPage } from "./workbench";
import "./styles/tokens.css";
import "./styles/workbench.css";

const root = document.getElementById("root");
if (!root) throw new Error("Workbench root is missing");
createRoot(root).render(<WorkbenchPage />);
