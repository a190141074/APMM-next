// 在 B 盘新建程序副本验证真实 Tauri IPC；所有 Mod 写入均指向测试用 modimg。
import { spawn } from "node:child_process";
import { chromium } from "playwright";
import {
  mkdirSync,
  readFileSync,
  writeFileSync,
  cpSync,
  existsSync,
  readdirSync,
  copyFileSync,
  renameSync,
} from "node:fs";
import { randomUUID, createHash } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const root = path.resolve(import.meta.dirname, "..");
const build = JSON.parse(
  readFileSync(path.join(root, "verification-portable-build.json"), "utf8"),
);
const plugin = JSON.parse(
  readFileSync(path.join(root, "verification-plugin-build.json"), "utf8"),
);
const output = path.join(
  process.env.APMM_WORK_DIR || "B:/APMM-next-work",
  "native-validation",
  randomUUID(),
);
const appDir = path.join(output, "app");
cpSync(build.payload, appDir, { recursive: true });
mkdirSync(path.join(appDir, "data"), { recursive: true });
writeFileSync(
  path.join(appDir, "data/library.json"),
  JSON.stringify({
    version: 1,
    settings: { updateAutoCheck: false },
    heroes: {},
    monsters: {},
  }),
);
const env = {
  ...process.env,
  APMM_WORK_DIR: "B:/APMM-next-work",
  TEMP: "B:/APMM-next-work/temp",
  TMP: "B:/APMM-next-work/temp",
  WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"),
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9422",
};
const child = spawn(path.join(appDir, "astral-mod-manager.exe"), [], {
  cwd: appDir,
  env,
  stdio: "ignore",
  windowsHide: true,
});
const checks = [];
const errors = [];
let browser;
let page;
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const hash = (file) =>
  createHash("sha256").update(readFileSync(file)).digest("hex");
try {
  for (let i = 0; i < 120; i++) {
    try {
      browser = await chromium.connectOverCDP("http://127.0.0.1:9422");
      break;
    } catch {
      await delay(500);
    }
  }
  assert.ok(browser, "桌面 WebView2 调试端口没有就绪");
  for (let i = 0; i < 80; i++) {
    page = browser
      .contexts()
      .flatMap((c) => c.pages())
      .find((p) => !p.url().startsWith("devtools:"));
    if (page) break;
    await delay(250);
  }
  assert.ok(
    page,
    "未找到 APMM WebView：" +
      JSON.stringify(
        await (await fetch("http://127.0.0.1:9422/json/list")).json(),
      ),
  );
  console.log("WebView URL", page.url());
  page.on("pageerror", (e) => errors.push(e.message));
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__);
  const call = (command, args = {}) =>
    page.evaluate(
      ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
      { command, args },
    );
  const task = async (kind, title, args = {}) => {
    const id = await call("task_execute", { kind, title, args });
    for (let i = 0; i < 1200; i++) {
      const value = (await call("tasks_list")).find((t) => t.id === id);
      if (value && !["queued", "running"].includes(value.status)) {
        assert.equal(value.status, "completed", JSON.stringify(value));
        return value.result;
      }
      await delay(200);
    }
    throw Error(`任务超时：${title}`);
  };
  await page.getByRole("button", { name: "先进入工作区", exact: true }).click();
  const initial = await call("workspace_snapshot");
  assert.equal(initial.version, build.version);
  assert.ok(initial.resources.length >= 2187);
  assert.equal(
    initial.resources.reduce((sum, r) => sum + r.files.length, 0),
    0,
  );
  checks.push("实际 EXE 启动与工作区 IPC");
  await task("aux-install", "验证离线插件安装", { path: plugin.zip });
  const installed = await call("auxiliary_status");
  assert.equal(installed.installed, true);
  assert.equal(installed.version, "0.1.0");
  checks.push("签名 ZIP 安装与版本指针");
  await call("settings_set", {
    patch: {
      modimgPath: "D:/Steam/steamapps/common/Astral Party/8vJXn6CN/modimg",
      pluginLocallowDir: "B:/feimo/feimo/AstralParty_CN",
    },
  });
  const checked = await task("aux-run", "只读检查实际游戏", {
    command: "check",
  });
  assert.equal(checked.status, "ok");
  checks.push("管理器启动独立插件并读取真实游戏");
  const extracted = await task("aux-run", "提取鼠标原图", {
    command: "sync",
    category: "cursor",
  });
  assert.equal(extracted.status, "ok");
  checks.push("内置资源鼠标双状态经插件生成");
  const virtualGame = path.join(output, "fixture-game/modimg");
  mkdirSync(virtualGame, { recursive: true });
  await call("settings_set", { patch: { modimgPath: virtualGame } });
  const fixtures = path.join("B:/APMM-next-work/fixtures/three-mods-10007");
  const paths = readdirSync(fixtures)
    .filter((n) => n.endsWith(".png"))
    .map((n) => path.join(fixtures, n));
  assert.equal(paths.length, 3);
  const target = {
    heroId: "10007",
    sectionKey: "hand_card",
    canonical: "UT_HandCard_10007",
  };
  const preview = await call("images_inspect", { paths, target });
  assert.ok(preview.every((r) => r.status === "new"));
  await task("images-import", "导入同槽位三个 Mod", {
    paths,
    target,
    expectedHashes: Object.fromEntries(preview.map((r) => [r.path, r.hash])),
  });
  const getRow = async () =>
    (await call("workspace_snapshot")).resources.find(
      (r) => r.canonical === target.canonical,
    );
  const row = await getRow();
  assert.equal(row.files.length, 3);
  assert.equal(row.activeHash, null);
  assert.equal(readdirSync(virtualGame).length, 0);
  checks.push("三文件导入保留独立身份且不自动应用");
  // 通过真实 React 页面触发应用队列，检查快速点击和文件消失后的状态恢复。
  await page.locator(".nav-item").filter({ hasText: "卡面" }).click();
  await page.getByRole("combobox", { name: "搜索资源" }).fill("10007");
  await page.getByRole("combobox", { name: "搜索资源" }).press("Enter");
  await page.locator(".detail-page").waitFor();
  const part = page.locator('[id="part-hand_card/UT_HandCard_10007"]');
  await part.locator(".candidate").nth(3).waitFor();
  assert.equal(await part.locator(".candidate").count(), 4);
  await part.locator(".candidate-apply").nth(1).click();
  await page.waitForFunction(
    () =>
      !document
        .querySelector(".detail-page .badge")
        ?.textContent.includes("处理中"),
  );
  await part.locator(".candidate-apply").evaluateAll((buttons) => {
    buttons[2].click();
    buttons[3].click();
    buttons[1].click();
  });
  await page.waitForFunction(
    () =>
      !document
        .querySelector(".detail-page .badge")
        ?.textContent.includes("处理中"),
  );
  assert.equal((await getRow()).activeHash, row.files[0].hash);
  assert.equal(
    hash(path.join(virtualGame, target.canonical + ".png")),
    row.files[0].hash,
  );
  checks.push("桌面详情三个候选、快速切换最后点击生效");
  const missing = row.files[2].path;
  assert.ok(path.resolve(missing).startsWith(path.resolve(appDir) + path.sep));
  renameSync(missing, missing + ".missing");
  try {
    await part.locator(".candidate-apply").nth(3).click();
    await page.waitForFunction(
      () =>
        !document
          .querySelector(".detail-page .badge")
          ?.textContent.includes("处理中"),
    );
    assert.equal((await getRow()).activeHash, row.files[0].hash);
    assert.equal(
      hash(path.join(virtualGame, target.canonical + ".png")),
      row.files[0].hash,
    );
  } finally {
    renameSync(missing + ".missing", missing);
  }
  checks.push("候选文件缺失时恢复真实应用状态");
  await part.locator(".candidate-apply").first().click();
  await page.waitForFunction(
    () =>
      !document
        .querySelector(".detail-page .badge")
        ?.textContent.includes("处理中"),
  );
  assert.equal((await getRow()).activeHash, null);
  assert.equal(
    existsSync(path.join(virtualGame, target.canonical + ".png")),
    false,
  );

  const fourth=path.join(output,"ui-import/UT_HandCard_10007.png");mkdirSync(path.dirname(fourth),{recursive:true});
  copyFileSync(path.join(appDir,"data/assets/outimg/cursor_up.png"),fourth);
  await call("plugin:event|emit_to",{target:{kind:"Window",label:"main"},event:"tauri://drag-drop",payload:{paths:[fourth],position:{x:600,y:400}}});
  const wizard=page.getByRole("dialog");await wizard.waitFor();
  await wizard.getByRole("button",{name:"下一步",exact:false}).click();
  await wizard.getByRole("button",{name:"确认导入",exact:true}).click();
  await wizard.getByRole("heading",{name:"处理完成",exact:true}).waitFor();
  assert.match(await wizard.locator(".package-result").innerText(),/成功 1/);
  await delay(200);await wizard.getByRole("button",{name:"返回工作区",exact:true}).click();
  await part.locator(".candidate").nth(4).waitFor();
  assert.equal((await getRow()).files.length,4);
  checks.push("真实导入向导等待完成并即时显示第四候选");
  await call("remove_file",{args:{...target,hash:hash(fourth)}});
  await page.screenshot({ path: path.join(output, "native-detail.png") });
  await page.getByRole("button", { name: "返回列表", exact: true }).click();
  checks.push("桌面点击原版还原实际游戏部位");
  const quick = await task("quick-export", "快速导出 PNG", {
    destination: path.join(output, "quick.png"),
    mode: "file",
    items: [
      {
        category: "cards",
        entityId: "10007",
        sectionKey: "hand_card",
        canonical: target.canonical,
        hash: row.files[0].hash,
      },
    ],
  });
  assert.equal(hash(quick.path), row.files[0].hash);
  checks.push("快速导出 PNG 保留原始字节");
  await call("resources_apply", {
    items: [
      {
        entityId: "10007",
        section: "hand_card",
        canonical: target.canonical,
        hash: row.files[1].hash,
      },
    ],
  });
  assert.equal(
    hash(path.join(virtualGame, target.canonical + ".png")),
    row.files[1].hash,
  );
  checks.push("明确应用写入隔离游戏目录");
  const exported = await task("package-export", "加密包往返导出", {
    destination: path.join(output, "roundtrip.apmod"),
    password: "acceptance-fixture",
    items: row.files.map((f) => ({
      category: "cards",
      entityId: "10007",
      sectionKey: "hand_card",
      canonical: target.canonical,
      hash: f.hash,
    })),
    includeApplicationState: true,
    maxVolumeBytes: 16 * 1024 * 1024,
  });
  for (const file of row.files)
    await call("remove_file", { args: { ...target, hash: file.hash } });
  const begin = await call("package_import_queue_begin");
  await assert.rejects(() =>
    call("package_import_queue_add", {
      args: {
        queueId: begin.queueId,
        path: exported.path,
        password: "wrong-password",
      },
    }),
  );
  checks.push("历史兼容导入入口拒绝错误密码");
  await call("package_import_queue_add", {
    args: {
      queueId: begin.queueId,
      path: exported.path,
      password: "acceptance-fixture",
    },
  });
  const queued = await call("package_import_queue_preview", {
    args: { queueId: begin.queueId },
  });
  assert.equal(queued.files.length, 3);
  await task("package-commit", "加密包往返导入", {
    queueId: begin.queueId,
    items: queued.files.map((f) => ({ id: f.id, applyMode: "importOnly" })),
    useApplicationState: false,
  });
  assert.deepEqual(
    (await getRow()).files.map((f) => f.hash).sort(),
    row.files.map((f) => f.hash).sort(),
  );
  checks.push("APMMOD03 三文件加密往返与队列提交");
  const pair = [];
  for (const canonical of ["cursor_up", "cursor_down"]) {
    const png = path.join(appDir, "data/assets/outimg", canonical + ".png");
    const t = { heroId: "cursor", sectionKey: "cursor", canonical };
    await task("images-import", "导入双状态验证图", {
      paths: [png],
      target: t,
    });
    const r = (await call("workspace_snapshot")).resources.find(
      (r) => r.canonical === canonical,
    );
    pair.push({
      entityId: "cursor",
      section: "cursor",
      canonical,
      hash: r.files[0].hash,
    });
  }
  await call("resources_apply", { items: pair });
  for (const r of pair)
    assert.equal(hash(path.join(virtualGame, r.canonical + ".png")), r.hash);
  checks.push("鼠标整组应用的实际文件");
  const preset = await call("preset_save", { name: "验收双状态" });
  await call("preset_restore_all");
  assert.equal(readdirSync(virtualGame).length, 0);
  await call("preset_apply", { id: preset.id });
  for (const r of pair)
    assert.equal(hash(path.join(virtualGame, r.canonical + ".png")), r.hash);
  checks.push("预设保存、全部还原与事务恢复");
  const snapshot = await call("workspace_snapshot");
  assert.ok(
    snapshot.resources.find((r) => r.canonical === "cursor_up").originalUrl,
  );
  checks.push("校验过的本地原图优先显示");
  await page.locator(".nav-item").filter({ hasText: "鼠标样式" }).click();
  await delay(350);
  await page
    .locator(".resource-card img")
    .first()
    .evaluate((img) => img.decode());
  assert.ok(
    readdirSync(path.join(appDir, "data/cache/thumbnails")).some((n) =>
      n.endsWith(".png"),
    ),
  );
  checks.push("原生缩略图协议生成独立缓存");
  await page.screenshot({ path: path.join(output, "native-cursors.png") });
  await page.getByRole("button", { name: "任务进程", exact: true }).click();
  await page.screenshot({ path: path.join(output, "native-tasks.png") });
  const original = path.join(appDir, "data/assets/outimg/cursor_up.png");
  const originalHash = hash(original);
  await call("auxiliary_uninstall");
  assert.equal((await call("auxiliary_status")).installed, false);
  assert.equal(hash(original), originalHash);
  checks.push("卸载辅助插件保留提取成果");
  const history = JSON.parse(
    readFileSync(path.join(appDir, "data/tasks/history.json"), "utf8"),
  );
  assert.ok(history.length >= 8);
  assert.ok(!JSON.stringify(history).includes("acceptance-fixture"));
  checks.push("任务历史持久化且不包含包密码");
  assert.deepEqual(errors, []);
  await call("plugin:window|close", { label: "main" }).catch(() => {});
} finally {
  writeFileSync(
    path.join(root, "verification-native.json"),
    JSON.stringify({ output, checks, errors, pid: child.pid }, null, 2),
  );
  if (page && !page.isClosed())
    await page
      .screenshot({ path: path.join(output, "last-state.png") })
      .catch(() => {});
  await browser?.close().catch(() => {});
  if (child.exitCode === null) child.kill();
}
console.log(JSON.stringify({ output, checks, errors }));
