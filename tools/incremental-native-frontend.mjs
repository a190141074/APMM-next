// 记录正式 WebView 的资源增量和页面主动重取；验收调用绕过计数，避免污染指标。
export async function observeRenderer(page) {
  await page.evaluate(async () => {
    window.__apmmProbe = {
      invoke: window.__TAURI_INTERNALS__.invoke,
      snapshots: 0,
      deltas: [],
    };
    const probe = window.__apmmProbe;
    window.__TAURI_INTERNALS__.invoke = (command, ...args) => {
      if (command === "workspace_snapshot") probe.snapshots++;
      return probe.invoke(command, ...args);
    };
    await probe.invoke("plugin:event|listen", {
      event: "workspace-updated",
      target: { kind: "Any" },
      handler: window.__TAURI_INTERNALS__.transformCallback((event) =>
        probe.deltas.push(event.payload),
      ),
    });
  });
}
