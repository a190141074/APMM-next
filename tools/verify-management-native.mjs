import { portableBuildReport } from "./work-paths.mjs";
import { createServer } from "node:net";
// 发行 EXE 的管理功能验收；游戏、库、浏览器资料和失败夹具全部使用 B 盘独立副本。
import { spawn } from "node:child_process";
import { chromium } from "playwright";
import {
  mkdirSync,
  readFileSync,
  writeFileSync,
  cpSync,
  renameSync,
  rmSync,
  existsSync,
  readdirSync,
  statSync,
  utimesSync,
} from "node:fs";
import { randomUUID, createHash } from "node:crypto";
import { png } from "./management-fixtures.mjs";
import { verifyManagementNativeUI } from "./management-native-ui.mjs";
import { verifyManagementAbout } from "./management-about-ui.mjs";
import { verifySamePathImageRefresh } from "./management-image-refresh.mjs";
import { verifyNativeTemporary } from "./management-native-temporary.mjs";
import path from "node:path";
import assert from "node:assert/strict";
const debugPort = await new Promise((resolve) => {
  const server = createServer();
  server.listen(0, "127.0.0.1", () => {
    const port = server.address().port;
    server.close(() => resolve(port));
  });
});
const root = path.resolve(import.meta.dirname, "..");
const build = JSON.parse(
  readFileSync(portableBuildReport, "utf8"),
);
const output = path.join("B:/APMM-next-work/management-native", randomUUID());
const appDir = path.join(output, "app"),
  game = path.join(output, "game/modimg"),
  inputs = path.join(output, "input");
cpSync(build.payload, appDir, { recursive: true });
for (const p of [game, inputs, path.join(appDir, "data")])
  mkdirSync(p, { recursive: true });
writeFileSync(
  path.join(appDir, "data/library.json"),
  JSON.stringify({
    version: 1,
    settings: {
      modimgPath: game,
      monitorDirectories: true,
      updateAutoCheck: false,
    },
    heroes: {},
    monsters: {},
  }),
);
const sha = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");
const a = path.join(inputs, "UT_Hero_Card_101.png"),
  b = path.join(inputs, "UT_Hero_Card2_101.png"),
  unknown = path.join(inputs, "unknown-art.png"),
  alt = path.join(inputs, "alternate.png"),
  invalid = path.join(inputs, "fake.png");
writeFileSync(a, png(40));
writeFileSync(b, png(41));
writeFileSync(unknown, png(42));
writeFileSync(alt, png(43));
writeFileSync(invalid, Buffer.from("invalid png"));
const A = {
  category: "heroes",
  heroId: "101",
  sectionKey: "card",
  canonical: "UT_Hero_Card_101",
};
const B = {
  category: "heroes",
  heroId: "101",
  sectionKey: "card2",
  canonical: "UT_Hero_Card2_101",
};
const C = {
  category: "cursors",
  heroId: "cursor",
  sectionKey: "cursor",
  canonical: "cursor_up",
};
const argsFor = (slot, hash) => ({
  heroId: slot.heroId,
  sectionKey: slot.sectionKey,
  canonical: slot.canonical,
  hash,
});
const checks = [],
  errors = [];
let browser, page, child;
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
const until = async (fn, label) => {
  for (let i = 0; i < 160; i++) {
    if (await fn()) return;
    await delay(250);
  }
  throw Error(`超时：${label}`);
};
try {
  child = spawn(path.join(appDir, "astral-mod-manager.exe"), [], {
    cwd: appDir,
    windowsHide: true,
    stdio: ["ignore", "ignore", "pipe"],
    env: {
      ...process.env,
      APMM_WORK_DIR: "B:/APMM-next-work",
      TEMP: "B:/APMM-next-work/temp",
      TMP: "B:/APMM-next-work/temp",
      WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
    },
  });
  await until(async () => {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);
      return true;
    } catch {
      return false;
    }
  }, "WebView2 启动");
  await until(async () => {
    page = browser
      .contexts()
      .flatMap((c) => c.pages())
      .find((p) => !p.url().startsWith("devtools:"));
    return !!page;
  }, "WebView 页面");
  child.on("exit", (code, signal) => console.log("EXE exit", { code, signal }));
  child.stderr?.on("data", (data) => console.error(String(data)));
  page.on("pageerror", (e) => errors.push(e.message));
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__);
  const call = (command, args = {}) =>
    page.evaluate(
      ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
      { command, args },
    );
  const task = async (kind, args) => {
    const id = await call("task_execute", {
      kind,
      title: "管理功能隔离验收",
      args,
    });
    let value;
    await until(async () => {
      value = (await call("tasks_list")).find((t) => t.id === id);
      return value && !["queued", "running"].includes(value.status);
    }, kind);
    assert.equal(value.status, "completed", JSON.stringify(value));
    return value.result;
  };
  const snapshot = () => call("workspace_snapshot");
  const row = async (slot) =>
    (await snapshot()).resources.find((r) => r.canonical === slot.canonical);
  const apply = async (slot, hash) =>
    call("resources_apply", {
      items: [
        {
          entityId: slot.heroId,
          section: slot.sectionKey,
          canonical: slot.canonical,
          hash,
        },
      ],
    });
  assert.equal((await snapshot()).version, "0.3.39");
  assert.equal(
    path.normalize((await snapshot()).dataDir),
    path.normalize(path.join(appDir, "data")),
  );
  checks.push("0.3.39 正式 EXE 与 IPC 启动");
  const inspected = await call("images_inspect", {
    paths: [a, b, unknown, invalid],
  });
  assert.equal(inspected[2].meta.sectionKey, "temporary");
  assert.equal(inspected[3].status, "error");
  await task("images-import", {
    paths: [a, b, unknown, alt],
    expectedHashes: Object.fromEntries(
      [a, b, unknown, alt].map((p) => [p, sha(p)]),
    ),
    targets: { [alt]: A },
  });
  assert.equal((await row(A)).files.length, 2);
  const hA = sha(a),
    hB = sha(b),
    hAlt = sha(alt),
    hUnknown = sha(unknown);
  const info = {
    provenance: {
      author: "作者 A",
      authorUrls: [
        "https://example.com/author1",
        "https://example.com/author2",
      ],
      artist: "画师 A",
      artistUrls: [
        "https://example.com/artist1",
        "https://example.com/artist2",
      ],
      sourceUrl: "https://example.com/mod",
      version: "1.2.3",
    },
    note: "只存本机的备注",
  };
  await call("candidate_info_set", { args: argsFor(A, hA), info });
  const fa = (await row(A)).files.find((f) => f.hash === hA);
  assert.equal(sha(fa.path), hA);
  assert.equal(
    (await row(A)).files.find((f) => f.hash === hAlt).info.provenance.author,
    "",
  );
  const same = path.join(inputs, "shared.png");
  writeFileSync(same, png(40));
  await task("images-import", {
    paths: [same],
    expectedHashes: { [same]: hA },
    targets: { [same]: B },
  });
  assert.equal(
    (await row(B)).files.find((f) => f.hash === hA).info.provenance.author,
    "",
  );
  checks.push(
    "有效未知 PNG、损坏拒绝、逐项归属、同槽位和跨槽位信息隔离、元信息不改哈希",
  );
  await verifyNativeTemporary({
    page,
    game,
    call,
    snapshot,
    until,
    task,
    sha,
    checks,
  });
  const pack = path.join(output, "shared.apmod");
  const exported = await task("package-export", {
    destination: pack,
    password: "native-pass",
    items: [
      {
        category: A.category,
        entityId: A.heroId,
        sectionKey: A.sectionKey,
        canonical: A.canonical,
        hash: hA,
      },
    ],
    includeApplicationState: false,
    maxVolumeBytes: 16 * 1024 * 1024,
  });
  const q = await call("package_import_queue_begin");
  await call("package_import_queue_add", {
    args: {
      queueId: q.queueId,
      path: exported.volumes[0] || exported.path,
      password: "native-pass",
    },
  });
  const preview = await call("package_import_queue_preview", {
    args: { queueId: q.queueId },
  });
  assert.equal(preview.files[0].meta.provenance.author, info.provenance.author);
  assert.deepEqual(
    preview.files[0].meta.provenance.authorUrls,
    info.provenance.authorUrls,
  );
  assert.equal(preview.files[0].meta.provenance.artist, info.provenance.artist);
  assert.deepEqual(
    preview.files[0].meta.provenance.artistUrls,
    info.provenance.artistUrls,
  );
  assert.equal(preview.files[0].meta.provenance.note, undefined);
  await call("candidate_info_set", {
    args: argsFor(A, hA),
    info: {
      provenance: {
        author: "本机优先",
        sourceUrl: "",
        version: "",
        artist: "本机画师",
        artistUrls: ["https://example.com/local-artist"],
      },
      note: "保留本机",
    },
  });
  await task("package-commit", {
    queueId: q.queueId,
    items: preview.files.map((f) => ({ id: f.id, applyMode: "importOnly" })),
    useApplicationState: false,
  });
  const merged = (await row(A)).files.find((f) => f.hash === hA).info;
  assert.equal(merged.provenance.author, "本机优先");
  assert.equal(merged.provenance.version, "1.2.3");
  assert.equal(merged.note, "保留本机");
  assert.equal(merged.provenance.artist, "本机画师");
  assert.deepEqual(merged.provenance.artistUrls, [
    "https://example.com/local-artist",
  ]);
  assert.deepEqual(merged.provenance.authorUrls, info.provenance.authorUrls);
  assert.equal(sha(fa.path), hA);
  checks.push("apmod 来源往返、备注不分享、重复导入本机信息优先并补空");
  await apply(A, hA);
  await apply(B, hB);
  await call("mod_set_disabled", { args: argsFor(B, hB), disabled: true });
  const local = await call("preset_create", {
    name: "固定局部",
    scope: "local",
    selected: [A, B],
  });
  assert.equal(local.items.find((i) => i.canonical === A.canonical).hash, hA);
  assert.equal(local.items.find((i) => i.canonical === B.canonical).hash, "");
  const global = await call("preset_create", {
    name: "整套搭配",
    scope: "global",
    selected: [],
  });
  assert.ok(global.items.length >= 2187);
  await call("preset_lock", { id: local.id, locked: true });
  await assert.rejects(call("preset_delete", { id: local.id }), /锁定/);
  await assert.rejects(
    call("preset_update", { id: local.id, name: "x", items: local.items }),
    /锁定/,
  );
  const copy = await call("preset_copy", { id: local.id });
  assert.equal(copy.locked, false);
  await call("preset_lock", { id: local.id, locked: false });
  writeFileSync(path.join(game, "cursor_up.png"), png(50));
  await until(
    async () =>
      (await row(C))?.activeHash === sha(path.join(game, "cursor_up.png")),
    "外部新增",
  );
  const cHash = sha(path.join(game, "cursor_up.png"));
  await apply(A, hAlt);
  const plan = await call("preset_detail", { id: local.id });
  assert.equal(plan.canApply, true);
  assert.equal(plan.changes.length, 2);
  assert.ok(plan.changes.every((c) => c.changed));
  await call("preset_apply", { id: local.id });
  assert.equal(sha(path.join(game, A.canonical + ".png")), hA);
  assert.ok(!existsSync(path.join(game, B.canonical + ".png.disabled")));
  assert.equal(sha(path.join(game, "cursor_up.png")), cHash);
  checks.push(
    "全局/固定局部捕获、禁用保存原版、范围外不变、锁定校验与未锁定副本",
  );
  await apply(A, hAlt);
  await call("preset_apply", { id: global.id });
  assert.equal(sha(path.join(game, A.canonical + ".png")), hA);
  assert.ok(!existsSync(path.join(game, "cursor_up.png")));
  assert.ok(!existsSync(path.join(game, B.canonical + ".png.disabled")));
  await apply(C, cHash);
  checks.push("正式全局预设整套应用，未保存替换槽位恢复原版");
  const size = readFileSync(fa.path).length;
  const previousStamp = statSync(fa.path);
  rmSync(path.join(game, A.canonical + ".png"));
  writeFileSync(fa.path, png(44));
  utimesSync(fa.path, previousStamp.atime, previousStamp.mtime);
  assert.equal(readFileSync(fa.path).length, size);
  const editedHash = sha(fa.path);
  await until(
    async () => (await row(A)).files.some((f) => f.hash === editedHash),
    "库内同秒同大小修改",
  );
  const edited = (await row(A)).files.find((f) => f.hash === editedHash);
  assert.equal(edited.info.provenance.author, "本机优先");
  assert.equal(edited.info.note, "保留本机");
  const missing = await call("preset_detail", { id: local.id });
  assert.equal(missing.canApply, false);
  assert.equal(
    missing.preset.items.find((i) => i.canonical === A.canonical).hash,
    hA,
  );
  assert.equal(
    missing.preset.items.find((i) => i.canonical === A.canonical).displayName,
    local.items.find((i) => i.canonical === A.canonical).displayName,
  );
  writeFileSync(path.join(game, A.canonical + ".png.part"), png(45));
  renameSync(
    path.join(game, A.canonical + ".png.part"),
    path.join(game, A.canonical + ".png"),
  );
  const externalHash = sha(path.join(game, A.canonical + ".png"));
  await until(
    async () => (await row(A)).activeHash === externalHash,
    "游戏原子保存",
  );
  assert.equal(
    (await row(A)).files.find((f) => f.hash === externalHash).info.provenance
      .author,
    "",
  );
  const repaired = local.items.map((i) =>
    i.canonical === A.canonical
      ? { ...i, hash: editedHash, displayName: edited.sourceName }
      : i,
  );
  await call("preset_update", {
    id: local.id,
    name: "已修复局部",
    items: repaired,
  });
  await call("preset_apply", { id: local.id });
  assert.equal(sha(path.join(game, A.canonical + ".png")), editedHash);
  checks.push(
    "监听强制重哈希、候选编辑继承信息、预设旧哈希保留、原子保存不凭槽位继承作者、修复后整批应用",
  );
  const renamed = path.join(path.dirname(edited.path), "renamed-native.png");
  renameSync(edited.path, renamed);
  await until(
    async () =>
      (await row(A)).files.find((f) => f.hash === editedHash)?.path &&
      path.normalize(
        (await row(A)).files.find((f) => f.hash === editedHash).path,
      ) === path.normalize(renamed),
    "外部库重命名",
  );
  assert.equal(
    (await row(A)).files.find((f) => f.hash === editedHash).info.note,
    "保留本机",
  );
  const T = {
    category: "heroes",
    heroId: `temporary:${hUnknown}`,
    sectionKey: "temporary",
    canonical: `unmatched:${hUnknown}`,
  };
  await call("mod_set_disabled", {
    args: argsFor(T, hUnknown),
    disabled: true,
  });
  await assert.rejects(
    call("candidate_classify", {
      args: argsFor(T, hUnknown),
      target: C,
      keepCurrent: false,
    }),
    /启用/,
  );
  await call("mod_set_disabled", {
    args: argsFor(T, hUnknown),
    disabled: false,
  });
  await call("candidate_info_set", {
    args: argsFor(T, hUnknown),
    info: {
      provenance: { author: "临时作者", sourceUrl: "", version: "" },
      note: "归类备注",
    },
  });
  await call("candidate_classify", {
    args: argsFor(T, hUnknown),
    target: C,
    keepCurrent: false,
  });
  for (let i = 0; i < 3; i++) await task("scan", {});
  assert.ok(
    !(await snapshot()).resources.some(
      (r) =>
        r.category === "temporary" && r.files.some((f) => f.hash === hUnknown),
    ),
  );
  assert.equal((await row(C)).activeHash, cHash);
  assert.equal(
    (await row(C)).files.find((f) => f.hash === hUnknown).info.note,
    "归类备注",
  );
  const fc = (await row(C)).files.find((f) => f.hash === hUnknown);
  const suggested = path.join(path.dirname(fc.path), B.canonical + ".png");
  renameSync(fc.path, suggested);
  await until(
    async () =>
      !!(await row(C)).files.find((f) => f.hash === hUnknown)?.classification
        .pending,
    "手动归属新规则冲突",
  );
  await call("candidate_classify", {
    args: argsFor(C, hUnknown),
    target: B,
    keepCurrent: true,
  });
  await task("scan", {});
  assert.equal(
    (await row(C)).files.find((f) => f.hash === hUnknown).classification
      .pending,
    null,
  );
  const newSuggestion = path.join(
    path.dirname(suggested),
    A.canonical + ".png",
  );
  renameSync(suggested, newSuggestion);
  await until(
    async () =>
      (await row(C)).files.find((f) => f.hash === hUnknown)?.classification
        .pending?.canonical === A.canonical,
    "不同规则再次确认",
  );
  await call("candidate_classify", {
    args: argsFor(C, hUnknown),
    target: A,
    keepCurrent: false,
  });
  assert.equal(
    (await row(A)).files.find((f) => f.hash === hUnknown).info.provenance
      .author,
    "临时作者",
  );
  checks.push(
    "库重命名保留信息、禁用临时保留、归类事务、重复扫描无临时副本、手动冲突保留/采用及同规则不重复提示",
  );
  const otherGame = path.join(output, "other-game/modimg");
  mkdirSync(otherGame, { recursive: true });
  await call("settings_set", { patch: { modimgPath: otherGame } });
  await until(
    async () => (await call("directory_sync_status")).paths.includes(otherGame),
    "切换路径重新监听",
  );
  writeFileSync(path.join(otherGame, "cursor_down.png"), png(55));
  await until(
    async () =>
      (await row({ canonical: "cursor_down" })).activeHash ===
      sha(path.join(otherGame, "cursor_down.png")),
    "新路径自动同步",
  );
  writeFileSync(path.join(game, "cursor_down.png"), png(56));
  await delay(1800);
  assert.equal(
    (await row({ canonical: "cursor_down" })).activeHash,
    sha(path.join(otherGame, "cursor_down.png")),
  );
  rmSync(path.join(otherGame, "cursor_down.png"));
  await until(
    async () => !(await row({ canonical: "cursor_down" })).activeHash,
    "外部删除",
  );
  await call("settings_set", { patch: { monitorDirectories: false } });
  await until(
    async () => !(await call("directory_sync_status")).enabled,
    "监听关闭",
  );
  writeFileSync(path.join(otherGame, "cursor_up.png"), png(57));
  await delay(1800);
  assert.notEqual(
    (await row(C)).activeHash,
    sha(path.join(otherGame, "cursor_up.png")),
  );
  await call("settings_set", {
    patch: { modimgPath: game, monitorDirectories: true },
  });
  await until(
    async () => (await call("directory_sync_status")).paths.includes(game),
    "监听重新启用",
  );
  await delay(4000);
  const rev = (await call("directory_sync_status")).revision;
  await delay(3000);
  assert.equal((await call("directory_sync_status")).revision, rev);
  checks.push(
    "路径切换释放旧监听、新路径增删、设置关闭/重新启用、自身事件最终安静",
  );
  // 用目录占位模拟索引提交失败，确认全部游戏替换文件回滚。
  await call("settings_set", { patch: { monitorDirectories: false } });
  await until(
    async () => !(await call("directory_sync_status")).enabled,
    "失败夹具停监听",
  );
  const file = path.join(appDir, "data/library.json"),
    before = readFileSync(path.join(game, A.canonical + ".png"));
  const part = path.join(appDir, "data/library.json.part");
  mkdirSync(part);
  await assert.rejects(apply(A, hAlt), /库|索引|暂存/);
  assert.deepEqual(readFileSync(path.join(game, A.canonical + ".png")), before);
  assert.ok(existsSync(file));
  rmSync(part, { recursive: true });
  checks.push("正式 IPC 索引提交失败回滚实际游戏文件");
  await verifyManagementNativeUI({
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
  });
  assert.deepEqual(errors, []);
  await call("settings_set", { patch: { monitorDirectories: true } });
  await until(
    async () => (await call("directory_sync_status")).enabled,
    "退出前恢复监听",
  );
  await verifySamePathImageRefresh({
    page,
    game,
    A,
    editedHash,
    row,
    until,
    sha,
    output,
    checks,
  });
  await verifyManagementAbout({ page, output, checks });
  writeFileSync(path.join(game, "cursor_down.png"), png(70));
  await page
    .getByRole("button", { name: "关闭", exact: true })
    .click()
    .catch(() => {});
  await until(async () => child.exitCode !== null, "主窗口正常退出");
  assert.equal(child.exitCode, 0);
  const afterExit = JSON.parse(
    readFileSync(path.join(appDir, "data/library.json"), "utf8"),
  );
  assert.notEqual(
    afterExit.heroes.cursor?.sections.cursor?.resources.cursor_down?.activeHash,
    sha(path.join(game, "cursor_down.png")),
  );
  checks.push("主窗口正常退出并取消待稳定目录事件，未继续提交索引");
} catch (error) {
  errors.push(error.stack || String(error));
  console.error(error);
  if (page && !page.isClosed()) {
    await page
      .screenshot({ path: path.join(output, "failure.png") })
      .catch(() => {});
    writeFileSync(
      path.join(output, "failure-ui.txt"),
      await page
        .locator("body")
        .ariaSnapshot()
        .catch(() => "页面已不可读取"),
    );
  }
} finally {
  await browser?.close().catch(() => {});
  child?.kill();
  const report = {
    version: build.version,
    executableSha256: sha(path.join(appDir, "astral-mod-manager.exe")),
    output,
    checks,
    errors,
    allPassed: !errors.length,
  };
  writeFileSync(
    path.join(root, "verification-management-native.json"),
    JSON.stringify(report, null, 2) + "\n",
  );
  console.log(JSON.stringify(report, null, 2));
}
if (errors.length) process.exitCode = 1;
