/** @aiPurpose Connect directly to the launch-selected scope and mount the investigation. */
import { ObservationApi } from "./api/client.js";

import "./app.css";

const connection = ObservationApi.connect();
void connection.then(async (api) => {
  const { mountInvestigation } = await import("./app.js");
  mountInvestigation(document.getElementById("app")!, api);
}).catch(() => {
  const root = document.getElementById("app")!;
  const title = document.createElement("h1");
  title.textContent = "无法连接调查服务";
  const help = document.createElement("p");
  help.textContent = "请确认调查服务正在运行且网络可达，然后刷新页面。";
  root.replaceChildren(title, help);
});
