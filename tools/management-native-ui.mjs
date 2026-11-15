// 正式 WebView 的管理页面验收；通过原生事件提供隔离路径，后续仍调用真实 Rust IPC。
import path from "node:path";
import assert from "node:assert/strict";
import { verifyModInfoUI } from "./management-mod-info-ui.mjs";
export async function verifyManagementNativeUI({
  page,
  call,
  row,
  output,
  unknown,
  alt,
  invalid,
  A,
  editedHash,
  renamed,
  sha,
  checks,
}) {
  const welcome = page.getByRole("button", {
    name: "先进入工作区",
    exact: true,
  });
  if (await welcome.count()) await welcome.click();
  await page.getByRole("button", { name: "预设", exact: true }).click();
  await page.getByRole("button", { name: "已修复局部", exact: true }).click();
  await page.getByRole("checkbox", { name: /显示全部保存范围/ }).check();
  await page.screenshot({ path: path.join(output, "presets-light.png") });
  await page.getByRole("button", { name: "编辑 / 修复", exact: true }).click();
  assert.ok(
    await page
      .getByRole("button", { name: "保存编辑", exact: true })
      .isEnabled(),
  );
  await page.screenshot({ path: path.join(output, "presets-edit-light.png") });
  await page.getByRole("button", { name: "取消编辑", exact: true }).click();
  await page.getByRole("button", { name: "角色", exact: true }).click();
  const search = page.getByRole("combobox", { name: "搜索资源" });
  await search.fill(A.canonical);
  await search.press("Enter");
  await page.locator(".detail-page").waitFor();
  const file = (await row(A)).files.find((f) => f.hash === editedHash);
  await page.getByTitle(`管理 ${file.sourceName}`, { exact: true }).click();
  await page.getByRole("dialog", { name: "模组信息", exact: true }).waitFor();
  assert.equal(
    await page.getByLabel("作者", { exact: true }).inputValue(),
    "本机优先",
  );
  await page
    .getByRole("textbox", { name: "备注（仅本机）", exact: true })
    .fill("正式 WebView 备注");
  await page.getByRole("button", { name: "保存信息", exact: true }).click();
  assert.equal(sha(renamed), editedHash);
  assert.equal(
    (await row(A)).files.find((f) => f.hash === editedHash).info.note,
    "正式 WebView 备注",
  );
  await page.screenshot({ path: path.join(output, "mod-info-light.png") });
  await verifyModInfoUI({
    page,
    output,
    checks,
    onRenamed: async (name) => {
      const changed = (await row(A)).files.find((f) => f.hash === editedHash);
      assert.equal(changed.sourceName, name);
      assert.equal(sha(renamed), editedHash);
    },
    onInfoSaved: async () => {
      let info;
      for (let i = 0; i < 40; i++) {
        info = (await row(A)).files.find((f) => f.hash === editedHash).info;
        if (info.provenance.artist === "独立画师") break;
        await page.waitForTimeout(100);
      }
      assert.equal(info.provenance.artist, "独立画师");
      assert.deepEqual(info.provenance.authorUrls, [
        "https://example.com/author1",
        "https://example.com/author2",
      ]);
      assert.deepEqual(info.provenance.artistUrls, [
        "https://example.com/artist1",
        "https://example.com/artist2",
      ]);
      assert.equal(info.note, "正式 WebView 备注");
      assert.equal(sha(renamed), editedHash);
    },
  });
  await page.setViewportSize({ width: 800, height: 880 });
  await page.evaluate(() => (document.documentElement.dataset.theme = "dark"));
  await page.waitForTimeout(220);
  await page.screenshot({
    path: path.join(output, "mod-info-narrow-dark.png"),
  });
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  checks.push("正式 WebView 模组信息编辑不改 PNG、浅深主题及窄窗口");
  await page.locator(".package-menu > summary").click();
  await page
    .locator(".package-menu")
    .getByRole("button", { name: /导入 PNG/ })
    .click();
  await page
    .getByRole("dialog", { name: "导入 PNG", exact: true })
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  // Tauri 的 invoke 属性不可改写，使用公开事件入口提供夹具，不替换安全桥接。
  await call("plugin:event|emit_to", {
    target: { kind: "Window", label: "main" },
    event: "tauri://drag-drop",
    payload: { paths: [unknown, alt, invalid], position: { x: 0, y: 0 } },
  });
  await page.getByRole("dialog", { name: "导入资源", exact: true }).waitFor();
  await page.getByRole("button", { name: /下一步/ }).click();
  const classification = page.locator(".package-classification").first();
  await classification.getByRole("combobox", { name: "归属分类" }).click();
  await page.getByRole("option", { name: "角色", exact: true }).click();
  const target = await row(A);
  await classification.getByRole("combobox", { name: "归属变体" }).click();
  await page
    .getByRole("option", { name: target.variantLabel, exact: true })
    .click();
  await classification.getByRole("combobox", { name: "归属部位" }).click();
  await page
    .getByRole("option", {
      name: `${target.resourceLabel || target.sectionLabel} · ${A.canonical}`,
      exact: true,
    })
    .click();
  await page.screenshot({
    path: path.join(output, "import-classification-narrow-dark.png"),
  });
  assert.ok(
    (await page.getByRole("combobox", { name: "归属分类" }).count()) >= 2,
  );
  assert.ok(await page.getByText("无法导入", { exact: true }).count());
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  checks.push(
    "正式导入向导真实 PNG/错误 IPC 与逐项归属选择，窄窗口深色可操作；通过合成原生事件提供隔离路径，系统文件选择及真实外拖未验收",
  );
  await page.evaluate(() => localStorage.setItem("apmm.theme", "dark"));
  await page.reload();
  await page.getByRole("button", { name: "预设", exact: true }).click();
  await page.getByRole("button", { name: "已修复局部", exact: true }).click();
  await page.getByRole("checkbox", { name: /显示全部保存范围/ }).check();
  await page.screenshot({ path: path.join(output, "presets-narrow-dark.png") });
  checks.push("正式 WebView 预设页浅深主题、窄窗口和编辑操作");
}
