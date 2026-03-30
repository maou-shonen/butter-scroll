import { mount } from "svelte";
import ConfirmAppFilter from "./lib/ConfirmAppFilter.svelte";
import "./app.css";

const app = mount(ConfirmAppFilter, {
  target: document.getElementById("app")!,
});

export default app;
