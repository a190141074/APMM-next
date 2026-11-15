// 使用真实旧版助手和旧程序文件，在独立目录验证首次桥接与助手交接；不调用旧安装。
import { spawn, spawnSync } from "node:child_process";
import {
  copyFileSync,
  cpSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
  existsSync,
} from "node:fs";
import { createHash, randomUUID } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const root = path.resolve(import.meta.dirname, "..");
const old = process.env.APMM_LEGACY_FIXTURE;
if (!old) throw Error("请指定只读的 APMM_LEGACY_FIXTURE 旧版备份目录");
const distribution = JSON.parse(
  readFileSync(path.join(root, "verification-distributions.json"), "utf8"),
);
const build = JSON.parse(
  readFileSync(path.join(root, "verification-portable-build.json"), "utf8"),
);
const work = process.env.APMM_WORK_DIR || "B:/APMM-next-work";
const output = path.join(work, "legacy-bridge-validation", randomUUID());
const app = path.join(output, "app"),
  payload = path.join(output, "payload");
mkdirSync(path.join(app, "data"), { recursive: true });
for (const entry of readdirSync(old, { withFileTypes: true }))
  if (entry.isFile())
    copyFileSync(path.join(old, entry.name), path.join(app, entry.name));
cpSync(distribution.bridge, payload, { recursive: true });
writeFileSync(
  path.join(app, "data/library.json"),
  JSON.stringify({
    version: 1,
    settings: { updateAutoCheck: false, monitorDirectories: false },
    heroes: {},
    monsters: {},
  }),
);
writeFileSync(path.join(app, "data/keep.txt"), "preserve-user-data");
writeFileSync(path.join(app, "unrelated.txt"), "preserve-unknown-file");
const sha = (f) => createHash("sha256").update(readFileSync(f)).digest("hex");
const oldExecutable = sha(path.join(app, "astral-mod-manager.exe"));
const oldHelper = sha(path.join(app, "astral-updater.exe"));
const ended = spawnSync("cmd.exe", ["/d", "/c", "exit", "0"], {
  windowsHide: true,
});
const child = spawn(
  path.join(app, "astral-updater.exe"),
  [payload, app, String(ended.pid)],
  {
    cwd: app,
    windowsHide: true,
    stdio: "ignore",
    env: {
      ...process.env,
      APMM_WORK_DIR: work,
      TEMP: path.join(work, "temp"),
      TMP: path.join(work, "temp"),
      WEBVIEW2_USER_DATA_FOLDER: path.join(output, "webview"),
    },
  },
);
const marker = path.join(app, "data/updates/bootstrap-version.json");
const deadline = Date.now() + 90000;
while (!existsSync(marker) && Date.now() < deadline)
  await new Promise((resolve) => setTimeout(resolve, 250));
assert.ok(existsSync(marker), "旧版桥接未生成就绪标记");
assert.equal(
  JSON.parse(readFileSync(marker, "utf8")).version,
  distribution.version,
);
const bridgeManifest = JSON.parse(
  readFileSync(path.join(distribution.bridge, "release-manifest.json"), "utf8"),
);
for (const file of bridgeManifest.files) {
  const target = path.join(app, path.basename(file.path));
  assert.equal(sha(target), file.sha256, file.path);
}
assert.equal(
  readFileSync(path.join(app, "data/keep.txt"), "utf8"),
  "preserve-user-data",
);
assert.equal(
  readFileSync(path.join(app, "unrelated.txt"), "utf8"),
  "preserve-unknown-file",
);
const report = {
  version: distribution.version,
  output,
  app,
  oldExecutable,
  oldHelper,
  newHelper: sha(path.join(app, "astral-updater.exe")),
  bootstrap: true,
  userDataPreserved: true,
  unknownFilePreserved: true,
  oldUpdateUiTested: false,
};
writeFileSync(
  path.join(root, "verification-legacy-bridge.json"),
  JSON.stringify(report, null, 2),
);
console.log(JSON.stringify(report));
child.unref();
