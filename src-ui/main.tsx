import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { App } from "./app/App";
import { AppearanceProvider } from "./hooks/useAppearance";
import "./style.css";
import "./workspace.css";
import "./compact-workspace.css";
import "./interface.css";
import "./management-ui.css";
import "./appearance.css";
import "./theme-editor.css";
import "./settings.css";
import "./workshop.css";
import "./overview.css";
import "./resource-list.css";
import "./resource-drop.css";
import "./scrollbars.css";
const client = new QueryClient({
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } },
});
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={client}>
      <AppearanceProvider>
        <App />
      </AppearanceProvider>
    </QueryClientProvider>
  </React.StrictMode>,
);
