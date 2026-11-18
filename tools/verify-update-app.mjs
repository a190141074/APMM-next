import { portableBuildReport, prepareParent, reportDir, work } from "./work-paths.mjs";
// 在独立 Windows 桌面上验证便携版原位更新、启动确认与数据保留。
import { spawn, spawnSync } from "node:child_process";
import { createServer } from "node:net";
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
const launcher = process.env.APMM_BACKGROUND_LAUNCHER;
const python = process.env.APMM_TEST_PYTHON;
assert.ok(launcher && python, "请指定能保持到更新结束的独立桌面启动器与 Python");
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
  work,
  "app-update-validation",
  randomUUID(),
);
assert.ok(path.resolve(output).toLowerCase().startsWith("b:\\apmm-next-work\\"));
const appDir = path.join(output, "app");
if (process.env.APMM_UPDATE_BASE_ZIP) {
  mkdirSync(appDir, { recursive: true });
  // 仅接受本地发行 ZIP 作为隔离夹具，并按发行清单移除便携包前缀。
  const extractPackage = [
    "import json",
    "import hashlib",
    "import sys",
    "import zipfile",
    "from pathlib import Path, PurePosixPath",
    "archive = zipfile.ZipFile(sys.argv[1])",
    "output = Path(sys.argv[2])",
    "names = archive.namelist()",
    "assert len(names) == len(set(names)), 'ZIP 包含重复路径'",
    "manifests = [name for name in names if PurePosixPath(name).name == 'release-manifest.json']",
    "assert len(manifests) == 1, 'ZIP 必须含有唯一发行清单'",
    "manifest_name = manifests[0]",
    "prefix = manifest_name[:-len('release-manifest.json')]",
    "prefix_path = PurePosixPath(prefix[:-1]) if prefix else PurePosixPath()",
    "assert not prefix or (len(prefix_path.parts) == 1 and prefix_path.parts[0] not in {'.', '..'} and chr(92) not in prefix and ':' not in prefix)",
    "manifest = json.loads(archive.read(manifest_name))",
    "records = {prefix + item['path']: item for item in manifest['files']}",
    "expected = set(records) | {manifest_name}",
    "assert set(names) == expected, 'ZIP 成员与发行清单不一致'",
    "assert len({item.casefold() for item in records}) == len(records), 'ZIP 路径在 Windows 上冲突'",
    "for name in expected:",
    "    relative = name[len(prefix):]",
    "    path = PurePosixPath(relative)",
    "    assert not path.is_absolute() and path.as_posix() == relative and '..' not in path.parts and chr(92) not in name and ':' not in relative",
    "    if name != manifest_name:",
    "        record = records[name]",
    "        data = archive.read(name)",
    "        assert len(data) == record['bytes'] and hashlib.sha256(data).hexdigest() == record['sha256'], '文件与发行清单摘要不符'",
    "    destination = output.joinpath(*path.parts)",
    "    destination.parent.mkdir(parents=True, exist_ok=True)",
    "    destination.write_bytes(archive.read(name))",
  ].join("\n");
  const unzip = spawnSync(
    path.join(root, "plugins/asset-tool/.venv/Scripts/python.exe"),
    ["-X", "utf8", "-c", extractPackage, process.env.APMM_UPDATE_BASE_ZIP, appDir],
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
const debugPort = await new Promise((resolve, reject) => {
  const server = createServer();
  server.once("error", reject);
  server.listen(0, "127.0.0.1", () => {
    const port = server.address().port;
    server.close(() => resolve(port));
  });
});
const env = {
  ...process.env,
  APMM_WORK_DIR: work,
  TEMP: path.join(work, "temp"),
  TMP: path.join(work, "temp"),
  WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"),
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=" + debugPort,
};
const child = spawn(
  python,
  [
    "-X", "utf8", "-B", launcher,
    path.join(appDir, "astral-mod-manager.exe"),
    path.join(output, "heartbeat.jsonl"),
  ],
  { cwd: appDir, env, stdio: ["pipe", "pipe", "inherit"], windowsHide: true },
);
child.stdout.resume();
child.stdin.on("error", () => {});
let launchError;
child.once("error", (error) => { launchError = error; });
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let browser;
async function connect() {
  for (let i = 0; i < 120; i++) {
    try {
      return await chromium.connectOverCDP("http://127.0.0.1:" + debugPort, { timeout: 1200 });
    } catch {
      if (launchError) throw launchError;
      if (child.exitCode !== null) throw new Error("独立桌面启动器提前退出：" + child.exitCode);
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
    prepareParent(path.join(reportDir, "verification-update-app.json")),
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
  child.stdin.end();
}
