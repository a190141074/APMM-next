// 从已核验的便携产物派生 MSI 和旧版桥接包；不读取或写入用户安装目录。
import { spawnSync } from "node:child_process";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  copyFileSync,
  readdirSync,
  statSync,
  symlinkSync,
} from "node:fs";
import { createHash, randomUUID } from "node:crypto";
import path from "node:path";
const root = path.resolve(import.meta.dirname, "..");
const build = JSON.parse(
  readFileSync(path.join(root, "verification-portable-build.json"), "utf8"),
);
const version = readFileSync(path.join(root, "VERSION"), "utf8").trim();
if (version !== build.version) throw Error("需要先构建当前版本便携包");
const work = process.env.APMM_WORK_DIR || "B:/APMM-next-work";
const stage = path.join(work, "distribution-build", randomUUID());
mkdirSync(stage, { recursive: true });
const hash = (file) =>
  createHash("sha256").update(readFileSync(file)).digest("hex");
const record = (directory, name) => ({
  path: name,
  bytes: statSync(path.join(directory, name)).size,
  sha256: hash(path.join(directory, name)),
});
for (const file of build.files) {
  if (hash(path.join(build.payload, file.path)) !== file.sha256)
    throw Error("便携产物摘要已变化");
}
const bridge = path.join(stage, "bridge");
mkdirSync(bridge);
// 0.3.39 的解包白名单固定在旧程序中，只能发送它已认识的文件；缺少安装标记按便携版处理。
const legacyAllowed = new Set([
  "astral-mod-manager.exe",
  "astral-mod-manager.exe.manifest",
  "WebView2Loader.dll",
  "api-ms-win-core-synch-l1-2-0.dll",
  "astral-updater.exe",
  "LICENSE.txt",
  "THIRD_PARTY_NOTICES.md",
]);
for (const file of build.files.filter((f) =>
  legacyAllowed.has(path.basename(f.path)),
))
  copyFileSync(
    path.join(build.payload, file.path),
    path.join(bridge, path.basename(file.path)),
  );
// 旧版不认识独立 NOTICE 文件，将归属说明一并保留在它支持的声明文件中。
writeFileSync(
  path.join(bridge, "THIRD_PARTY_NOTICES.md"),
  readFileSync(path.join(build.payload, "licenses/NOTICE.md"), "utf8") +
    "\n\n" +
    readFileSync(
      path.join(build.payload, "licenses/THIRD_PARTY_NOTICES.md"),
      "utf8",
    ),
);
copyFileSync(
  path.join(bridge, "astral-updater.exe"),
  path.join(bridge, "astral-updater-next.exe"),
);
const bridgeFiles = readdirSync(bridge).map((name) => record(bridge, name));
writeFileSync(
  path.join(bridge, "release-manifest.json"),
  JSON.stringify(
    { schema: 1, version, dataSchema: 1, files: bridgeFiles },
    null,
    2,
  ),
);
const bridgeZip = path.join(
  root,
  "dist",
  `APMM-${version}-windows-x86_64-legacy-bridge.zip`,
);
const archive = spawnSync(
  path.join(root, "plugins/asset-tool/.venv/Scripts/python.exe"),
  ["-X", "utf8", "-B", "tools/archive.py", bridge, bridgeZip],
  { cwd: root, windowsHide: true, stdio: "inherit" },
);
if (archive.status !== 0) throw Error("旧版桥接包生成失败");

// 已打包的 0.3.39 尚未包含 next 助手白名单，使用独立兼容包，由它自己的 runner 替换助手。
const compat = path.join(stage, "compat-0.3.39");
mkdirSync(compat);
const compatFiles = bridgeFiles.filter(f => f.path !== "astral-updater-next.exe");
for (const file of compatFiles) copyFileSync(path.join(bridge, file.path), path.join(compat, file.path));
writeFileSync(path.join(compat, "release-manifest.json"), JSON.stringify({schema:1, version, dataSchema:1, files:compatFiles}, null, 2));
const compatZip = path.join(root, "dist", `APMM-${version}-windows-x86_64-compat-0.3.39.zip`);
const compatArchive = spawnSync(path.join(root, "plugins/asset-tool/.venv/Scripts/python.exe"),
  ["-X", "utf8", "-B", "tools/archive.py", compat, compatZip], {cwd:root, windowsHide:true, stdio:"inherit"});
if (compatArchive.status !== 0) throw Error("0.3.39 兼容包生成失败");

const msiPayload = path.join(stage, "msi-payload");
mkdirSync(msiPayload);
// WiX 资源映射使用同卷相对入口，实际暂存文件仍在工作盘，避免盘符被当成 URI。
const resourceLink = path.join(
  root,
  "src-tauri/target-msvc",
  `msi-resources-${randomUUID()}`,
);
symlinkSync(msiPayload, resourceLink, "junction");
const resourcePath = (name) =>
  path
    .relative(path.join(root, "src-tauri"), path.join(resourceLink, name))
    .replaceAll("\\", "/");
const resources = {};
for (const file of build.files) {
  const target = path.join(msiPayload, file.path);
  mkdirSync(path.dirname(target), { recursive: true });
  copyFileSync(path.join(build.payload, file.path), target);
  if (!["astral-mod-manager.exe", "astral-updater.exe"].includes(file.path))
    resources[resourcePath(file.path)] = file.path;
}
writeFileSync(
  path.join(msiPayload, "installation.json"),
  JSON.stringify({ schema: 1, kind: "msi" }),
);
const msiFiles = build.files.map((f) => record(msiPayload, f.path));
writeFileSync(
  path.join(msiPayload, "release-manifest.json"),
  JSON.stringify(
    { schema: 1, version, dataSchema: 1, files: msiFiles },
    null,
    2,
  ),
);
resources[resourcePath("release-manifest.json")] = "release-manifest.json";
const config = path.join(stage, "bundle.json");
writeFileSync(
  config,
  JSON.stringify({
    bundle: {
      active: true,
      useLocalToolsDir: true,
      targets: ["msi"],
      resources,
      windows: {
        allowDowngrades: false,
        wix: {
          upgradeCode: "DF7031A2-E41A-465C-BCBA-4DF3FCB73894",
          language: "zh-CN",
        },
        webviewInstallMode: { type: "downloadBootstrapper", silent: true },
      },
    },
  }),
);
const env = {
  ...process.env,
  APMM_WORK_DIR: work,
  TEMP: path.join(work, "temp"),
  TMP: path.join(work, "temp"),
  TAURI_BUNDLER_CACHE: path.join(work, "tauri-bundler-cache"),
  CARGO_TARGET_DIR: path.join(root, "src-tauri/target-msvc"),
  PATH: `${process.env.APMM_RUST_BIN || "B:/Rust/rustup/toolchains/stable-x86_64-pc-windows-msvc/bin"};${process.env.PATH}`,
};
const result = spawnSync(
  process.execPath,
  [
    "node_modules/@tauri-apps/cli/tauri.js",
    "bundle",
    "--bundles",
    "msi",
    "--config",
    config,
    "--ci",
  ],
  { cwd: root, env, windowsHide: true, stdio: "inherit" },
);
if (result.status !== 0) throw Error("MSI 构建失败");
const msiDir = path.join(root, "src-tauri/target-msvc/release/bundle/msi");
const generated = readdirSync(msiDir).filter(
  (name) => name.includes(version) && name.endsWith(".msi"),
);
if (generated.length !== 1) throw Error("MSI 产物不唯一");
const msi = path.join(root, "dist", `APMM-${version}-windows-x86_64.msi`);
copyFileSync(path.join(msiDir, generated[0]), msi);
const artifacts = [
  {
    kind: "portable",
    path: build.zip,
    bytes: build.bytes,
    sha256: build.sha256,
  },
  ...[
    { kind: "legacy-bridge", path: bridgeZip },
    { kind: "compat-0.3.39", path: compatZip },
    { kind: "msi", path: msi },
  ].map((f) => ({ ...f, bytes: statSync(f.path).size, sha256: hash(f.path) })),
];
for (const file of artifacts)
  writeFileSync(
    file.path + ".sha256",
    `${file.sha256}  ${path.basename(file.path)}\n`,
  );
writeFileSync(
  path.join(root, "verification-distributions.json"),
  JSON.stringify(
    { version, stage, bridge, msiPayload, msiFiles, artifacts },
    null,
    2,
  ),
);
console.log(JSON.stringify({ version, stage, artifacts }));
