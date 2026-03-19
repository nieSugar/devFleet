import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("找不到根元素！请确保 HTML 中有 id=\"root\" 的元素。");
}

const root = createRoot(container);
root.render(<App />);
