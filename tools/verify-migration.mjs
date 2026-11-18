import { portableBuildReport } from "./work-paths.mjs";
// 在新的便携副本中复制真实旧库；不调用任何应用 Mod 的命令。
import { spawn } from "node:child_process";
import { chromium } from "playwright";
import { cpSync, mkdirSync, readFileSync, writeFileSync, readdirSync, existsSync } from "node:fs";
import { randomUUID, createHash } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const root = path.resolve(import.meta.dirname, "..");
const build = JSON.parse(readFileSync(portableBuildReport, "utf8"));
const source = process.env.APMM_OLD_DATA || "D:/MOD/APMM/astral-tauri/dist/AstralPartyModManager/data";
const output = path.join(process.env.APMM_WORK_DIR || "B:/APMM-next-work", "migration-validation", randomUUID());
const appDir = path.join(output, "app");
cpSync(build.payload, appDir, { recursive: true });
mkdirSync(path.join(appDir, "data"), { recursive: true });
writeFileSync(path.join(appDir, "data/library.json"), JSON.stringify({ version: 1, heroes: {}, monsters: {}, settings: { updateAutoCheck: false } }));
const digest = data => createHash("sha256").update(data).digest("hex");
const before = digest(readFileSync(path.join(source, "library.json")));
const old = JSON.parse(readFileSync(path.join(source, "library.json"), "utf8"));
const files = lib => [lib.heroes, lib.monsters].flatMap(map => Object.values(map || {}).flatMap(entity => Object.values(entity.sections || {}).flatMap(section => Object.values(section.resources || {}).flatMap(resource => resource.files || []))));
const game = old.settings?.modimgPath;
const gameFiles = () => game && existsSync(game) ? Object.fromEntries(readdirSync(game, { withFileTypes: true }).filter(e => e.isFile()).map(e => [e.name, digest(readFileSync(path.join(game, e.name)))])) : {};
const beforeGame = gameFiles();
const env = { ...process.env, APMM_WORK_DIR: "B:/APMM-next-work", TEMP: "B:/APMM-next-work/temp", TMP: "B:/APMM-next-work/temp", WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"), WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9423" };
const child = spawn(path.join(appDir, "astral-mod-manager.exe"), [], { cwd: appDir, env, stdio: "ignore", windowsHide: true });
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
let browser;
try {
  for (let i = 0; i < 120; i++) { try { browser = await chromium.connectOverCDP("http://127.0.0.1:9423"); break; } catch { await delay(500); } }
  assert.ok(browser);
  const page = browser.contexts().flatMap(c => c.pages())[0];
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__);
  const call = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args });
  await page.getByRole("button", { name: "先进入工作区", exact: true }).click();
  const preview = await call("migration_inspect", { source });
  assert.ok(preview.mods > 0);
  const id = await call("task_execute", { kind: "migration", title: "真实旧库隔离迁移", args: { source } });
  let completed;
  for (let i = 0; i < 1800; i++) {
    const task = (await call("tasks_list")).find(t => t.id === id);
    if (task && !["queued", "running"].includes(task.status)) { assert.equal(task.status, "completed", JSON.stringify(task)); completed = task; break; }
    await delay(200);
  }
  assert.ok(completed, "迁移任务超时");
  const destination = path.join(appDir, "data");
  const actual = JSON.parse(readFileSync(path.join(destination, "library.json"), "utf8"));
  assert.equal(files(actual).length, files(old).length);
  let verifiedPayloads = 0;
  for (const entry of files(actual)) {
    const relative = entry.relPath;
    if (relative && existsSync(path.join(source, "vault", relative))) {
      assert.equal(digest(readFileSync(path.join(destination, "vault", relative))), entry.hash);
      verifiedPayloads++;
    }
  }
  assert.equal(digest(readFileSync(path.join(source, "library.json"))), before);
  assert.deepEqual(gameFiles(), beforeGame);
  assert.equal((await call("workspace_snapshot")).migrated, true);
  const report = { output, source, files: preview.files, bytes: preview.bytes, mods: preview.mods, verifiedPayloads, sourceLibraryUnchanged: true, gameFilesUnchanged: true, result: completed.result };
  writeFileSync(path.join(root, "verification-migration-native.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
} finally {
  await browser?.close().catch(() => {});
  if (child.exitCode === null) child.kill();
}
