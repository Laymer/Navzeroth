import { ReferenceView } from "./screens/ReferenceView";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { defaultClient } from "./commands";
import "./styles.css";

const root = document.getElementById("root");
if (!root) throw new Error("missing #root");
createRoot(root).render(
  <StrictMode>
    {window.location.hash === "#reference" ? <ReferenceView client={defaultClient()} /> : <App client={defaultClient()} />}
  </StrictMode>,
);
