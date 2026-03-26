import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

try {
  const app = mount(App, {
    target: document.getElementById("app")!,
  });
} catch (e) {
  const el = document.getElementById("__err");
  if (el) el.textContent = "[mount error] " + String(e);
  console.error("Failed to mount app:", e);
}
