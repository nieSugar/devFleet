import "@fontsource/plus-jakarta-sans/400.css";
import "@fontsource/plus-jakarta-sans/500.css";
import "@fontsource/plus-jakarta-sans/600.css";
import "@fontsource/plus-jakarta-sans/700.css";
import "@fontsource/plus-jakarta-sans/800.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";

import { createRoot } from "react-dom/client";
import App from "./App";
import "./i18n";
import "./index.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Root element not found. Ensure the HTML has an element with id=\"root\".");
}

const root = createRoot(container);
root.render(<App />);
