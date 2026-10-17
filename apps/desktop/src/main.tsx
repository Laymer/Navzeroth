import { ReferenceView } from "./screens/ReferenceView";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { defaultClient } from "./commands";
import { Backdrop } from "./theme/Backdrop";
import "./theme/tokens.css";
import "./theme/fonts.css";
import "./theme/base.css";
import "./ui/ui.css";
import "./styles.css";

const root = document.getElementById("root");
if (!root) throw new Error("missing #root");
createRoot(root).render(
  <StrictMode>
    <Backdrop />
    {window.location.hash === "#reference" ? (
      <div className="reference-shell"><ReferenceView client={defaultClient()} /></div>
    ) : (
      <App client={defaultClient()} />
    )}
  </StrictMode>,
);
