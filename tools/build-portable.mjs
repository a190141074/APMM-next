// 前端、发行暂存均位于专用工作盘；只允许清单内文件进入主程序 ZIP。
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  readFileSync,
  writeFileSync,
  readdirSync,
  statSync,
  copyFileSync,
  existsSync,
} from "node:fs";
import { createHash, randomUUID } from "node:crypto";
import path from "node:path";
const root = path.resolve(import.meta.dirname, "..");
const work = process.env.APMM_WORK_DIR || "B:/APMM-next-work";
const stage = path.resolve(work, "portable-build", randomUUID());
if (stage.startsWith(root + path.sep) || stage === root)
  throw Error("发行暂存目录必须在源码目录以外");
const frontend = path.join(stage, "frontend");
const payload = path.join(stage, "APMM");
mkdirSync(payload, { recursive: true });
mkdirSync(path.join(work, "temp"), { recursive: true });
const env = {
  ...process.env,
  APMM_WORK_DIR: work,
  TEMP: path.join(work, "temp"),
  TMP: path.join(work, "temp"),
};
function run(args, extra = {}) {
  const r = spawnSync(process.execPath, args, {
    cwd: root,
    env: { ...env, ...extra },
    stdio: "inherit",
  });
  if (r.status !== 0) throw Error(`构建失败：${args.join(" ")}`);
}
const version = readFileSync(path.join(root, "VERSION"), "utf8").trim();
const pkg = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8"));
const config = JSON.parse(
  readFileSync(path.join(root, "src-tauri/tauri.conf.json"), "utf8"),
);
if (pkg.version !== version || config.version !== version)
  throw Error("版本文件不一致");
if (!readFileSync("CHANGELOG.md", "utf8").includes(version))
  throw Error("缺少当前版本更新说明");
run(["tools/sync-ipc.mjs"]);
run(["node_modules/typescript/bin/tsc", "--noEmit"]);
run(["node_modules/vite/bin/vite.js", "build", "--outDir", frontend]);
const files = (dir) =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? files(path.join(dir, e.name)) : [path.join(dir, e.name)],
  );
const frontendFiles = files(frontend);
if (
  frontendFiles.some((f) =>
    /previews|\.tsx?$|\.map$|node_modules/i.test(path.relative(frontend, f)),
  )
)
  throw Error("前端产物混入源码或原图");
run(["tools/cargo.mjs", "build"], {
  // Windows 盘符会被 Tauri 的 URL 分支识别为协议，扩展路径明确表示本地目录。
  TAURI_CONFIG: JSON.stringify({
    build: { frontendDist: path.toNamespacedPath(frontend) },
  }),
});
const dependencies = readFileSync(
  path.join(root, "src-tauri/target-msvc/release/astral-mod-manager.d"),
  "utf8",
);
if (
  !dependencies.includes("index.html") ||
  /[\\/]src[\\/]previews[\\/]/i.test(dependencies)
)
  throw Error("发行二进制未正确嵌入构建后的前端或混入预览源图");
const names = [
  "astral-mod-manager.exe",
  "astral-updater.exe",
  "astral-mod-manager.exe.manifest",
  "WebView2Loader.dll",
  "api-ms-win-core-synch-l1-2-0.dll",
  "licenses/LICENSE.txt",
  "licenses/THIRD_PARTY_NOTICES.md",
  "licenses/NOTICE.md",
];
for (const name of names) {
  const source = name.endsWith(".exe")
    ? path.join(root, "src-tauri/target-msvc/release", name)
    : name.endsWith(".dll")
      ? path.join(root, "vendor/runtime", name)
      : name.endsWith(".manifest")
        ? path.join(root, "src-tauri", name)
        : path.join(root, path.basename(name));
  mkdirSync(path.dirname(path.join(payload, name)), { recursive: true });
  copyFileSync(source, path.join(payload, name));
}
const hash = (f) => createHash("sha256").update(readFileSync(f)).digest("hex");
writeFileSync(path.join(payload, "installation.json"), JSON.stringify({ schema: 1, kind: "portable" }));
names.push("installation.json");
const manifest = {
  schema: 1,
  version,
  dataSchema: 1,
  files: names.map((name) => ({
    path: name,
    bytes: statSync(path.join(payload, name)).size,
    sha256: hash(path.join(payload, name)),
  })),
};
writeFileSync(
  path.join(payload, "release-manifest.json"),
  JSON.stringify(manifest, null, 2) + "\n",
);
const audit = files(payload).map((f) =>
  path.relative(payload, f).replaceAll("\\", "/"),
);
if (audit.some((f) => !names.includes(f) && f !== "release-manifest.json"))
  throw Error("发行包包含非程序文件");
const dist = path.join(root, "dist");
mkdirSync(dist, { recursive: true });
const zip = path.join(dist, `APMM-${version}-windows-x86_64-portable.zip`);
const python = path.join(root, "plugins/asset-tool/.venv/Scripts/python.exe");
const archive = spawnSync(
  python,
  ["-X", "utf8", "-B", "tools/archive.py", payload, zip],
  { cwd: root, env, stdio: "inherit" },
);
if (archive.status !== 0) throw Error("ZIP 生成失败");
const report = {
  version,
  stage,
  payload,
  zip,
  bytes: statSync(zip).size,
  sha256: hash(zip),
  files: manifest.files,
  frontend: frontendFiles.map((f) => ({
    path: path.relative(frontend, f),
    bytes: statSync(f).size,
  })),
  excludes: ["用户数据", "游戏原图", "Python", "资产插件运行时", "前端源码"],
};
writeFileSync(
  path.join(root, "verification-portable-build.json"),
  JSON.stringify(report, null, 2),
);
writeFileSync(
  zip + ".sha256",
  report.sha256 + "  " + path.basename(zip) + "\n",
);
console.log(
  JSON.stringify({ zip, bytes: report.bytes, sha256: report.sha256 }),
);
