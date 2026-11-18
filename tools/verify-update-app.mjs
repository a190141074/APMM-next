import { portableBuildReport } from "./work-paths.mjs";
// 实际 APMM 在隔离目录执行同版本更新，验证 UI 就绪确认与数据保留的完整握手。
import { spawn, spawnSync } from "node:child_process";
import { chromium } from "playwright";
import {
  cpSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  existsSync,
} from "node:fs";
import { randomUUID, createHash } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const root = path.resolve(import.meta.dirname, "..");
const build = JSON.parse(
  readFileSync(portableBuildReport, "utf8"),
);
const incoming = process.env.APMM_UPDATE_INCOMING_ZIP;
if (incoming) {
  build.zip = incoming;
  build.bytes = readFileSync(incoming).length;
  build.sha256 = createHash("sha256")
    .update(readFileSync(incoming))
    .digest("hex");
}
const output = path.join(
  process.env.APMM_WORK_DIR || "B:/APMM-next-work",
  "app-update-validation",
  randomUUID(),
);
const appDir = path.join(output, "app");
if (process.env.APMM_UPDATE_BASE_ZIP) {
  mkdirSync(appDir, { recursive: true });
  // 仅接受本地已构建的 ZIP 作为旧版夹具，展开位置始终是新建隔离目录。
  const unzip = spawnSync(
    path.join(root, "plugins/asset-tool/.venv/Scripts/python.exe"),
    ["-m", "zipfile", "-e", process.env.APMM_UPDATE_BASE_ZIP, appDir],
    { windowsHide: true, encoding: "utf8" },
  );
  assert.equal(unzip.status, 0, unzip.stderr);
} else cpSync(build.payload, appDir, { recursive: true });
mkdirSync(path.join(appDir, "data/updates"), { recursive: true });
writeFileSync(
  path.join(appDir, "data/library.json"),
  JSON.stringify({
    version: 1,
    heroes: {},
    monsters: {},
    settings: { updateAutoCheck: false },
  }),
);
writeFileSync(path.join(appDir, "data/keep.txt"), "user-data-must-survive");
writeFileSync(path.join(appDir, "unrelated.txt"), "not-a-release-file");
writeFileSync(
  path.join(appDir, "data/updates/download-receipt.json"),
  JSON.stringify({
    version: build.version,
    path: build.zip,
    bytes: build.bytes,
    sha256: build.sha256,
  }),
);
const env = {
  ...process.env,
  APMM_WORK_DIR: "B:/APMM-next-work",
  TEMP: "B:/APMM-next-work/temp",
  TMP: "B:/APMM-next-work/temp",
  WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"),
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9424",
};
const child = spawn(path.join(appDir, "astral-mod-manager.exe"), [], {
  cwd: appDir,
  env,
  stdio: "ignore",
  windowsHide: true,
});
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let browser;
async function connect() {
  for (let i = 0; i < 120; i++) {
    try {
      return await chromium.connectOverCDP("http://127.0.0.1:9424");
    } catch {
      await delay(500);
    }
  }
  throw Error("更新验收的 WebView 未启动");
}
try {
  browser = await connect();
  const page = browser.contexts().flatMap((c) => c.pages())[0];
  await page.getByRole("button", { name: "先进入工作区", exact: true }).click();
  const before = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("workspace_snapshot"),
  );
  let applyError;
  await page
    .evaluate(
      ({ version, assetPath }) =>
        window.__TAURI_INTERNALS__.invoke("update_apply", {
          version,
          assetPath,
        }),
      { version: build.version, assetPath: build.zip },
    )
    .catch((e) => {
      applyError = String(e);
    });
  if (
    applyError &&
    !/closed|destroyed|navigation|disconnected/i.test(applyError)
  )
    throw Error(applyError);
  await browser.close().catch(() => {});
  let job;
  for (let i = 0; i < 300; i++) {
    const pending = path.join(appDir, "data/updates/pending.json");
    if (existsSync(pending)) {
      const { job: jobPath } = JSON.parse(readFileSync(pending, "utf8"));
      if (existsSync(jobPath)) {
        job = JSON.parse(readFileSync(jobPath, "utf8"));
        if (
          [
            "complete",
            "rolled-back",
            "recovery-required",
            "failed-before-apply",
          ].includes(job.status)
        )
          break;
      }
    }
    await delay(250);
  }
  assert.equal(job?.status, "complete", JSON.stringify(job));
  assert.equal(
    readFileSync(path.join(appDir, "data/keep.txt"), "utf8"),
    "user-data-must-survive",
  );
  assert.equal(
    readFileSync(path.join(appDir, "unrelated.txt"), "utf8"),
    "not-a-release-file",
  );
  const installedManifest = JSON.parse(
    readFileSync(path.join(appDir, "release-manifest.json"), "utf8"),
  );
  for (const file of installedManifest.files) {
    const hash = createHash("sha256")
      .update(readFileSync(path.join(appDir, file.path)))
      .digest("hex");
    assert.equal(hash, file.sha256);
  }
  browser = await connect();
  const restarted = browser.contexts().flatMap((c) => c.pages())[0];
  const snapshot = await restarted.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("workspace_snapshot"),
  );
  assert.equal(snapshot.version, build.version);
  const report = {
    output,
    status: job.status,
    version: snapshot.version,
    fromVersion: before.version,
    sameVersionFixture: before.version === build.version,
    userDataPreserved: true,
    unrelatedFilePreserved: true,
    startupAcknowledged: true,
  };
  writeFileSync(
    path.join(root, "verification-update-app.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report));
  await restarted
    .evaluate(() =>
      window.__TAURI_INTERNALS__.invoke("plugin:window|close", {
        label: "main",
      }),
    )
    .catch(() => {});
} finally {
  await browser?.close().catch(() => {});
  if (child.exitCode === null) child.kill();
}
