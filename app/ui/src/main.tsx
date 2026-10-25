// SPDX-License-Identifier: Apache-2.0
import { render } from "solid-js/web";
import { App } from "./App";
import { initTheme } from "./theme/theme";
import "./styles.css";
import "./layout.css";

async function start() {
  const inTauri = "__TAURI_INTERNALS__" in window;
  if (import.meta.env.DEV && !inTauri) {
    const { installDevMock } = await import("./devMock");
    await installDevMock(new URLSearchParams(location.search).get("fixture") ?? "demo");
  }
  initTheme();
  render(() => <App />, document.getElementById("root")!);
}
start();
