// 独立插件产物只包含运行程序与依赖，暂存、私钥和编译缓存全部位于专用工作盘。
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  readFileSync,
  writeFileSync,
  readdirSync,
  statSync,
  copyFileSync,
  cpSync,
} from "node:fs";
import { createHash, sign, randomUUID, createPublicKey } from "node:crypto";
import path from "node:path";
const root = path.resolve(import.meta.dirname, "..");
const work = process.env.APMM_WORK_DIR || "B:/APMM-next-work";
const stage = path.join(work, "plugin-build", randomUUID());
mkdirSync(stage, { recursive: true });
const python = path.join(root, "plugins/asset-tool/.venv/Scripts/python.exe");
const temp = path.join(work, "temp");
mkdirSync(temp, { recursive: true });
const env = {
  ...process.env,
  APMM_WORK_DIR: work,
  TEMP: temp,
  TMP: temp,
  PYTHONUTF8: "1",
};
const run = (args) => {
  const r = spawnSync(python, ["-X", "utf8", "-B", ...args], {
    cwd: root,
    env,
    stdio: "inherit",
  });
  if (r.status !== 0) throw Error(`构建失败：${args[0]}`);
};
run(["tools/sync-definitions.py"]);
run([
  "-m",
  "PyInstaller",
  "--noconfirm",
  "--onedir",
  "--name",
  "apmm-asset-tool",
  "--contents-directory",
  "runtime",
  "--distpath",
  path.join(stage, "dist"),
  "--workpath",
  path.join(stage, "build"),
  "--specpath",
  stage,
  "--paths",
  "plugins/asset-tool/src",
  "--collect-all",
  "UnityPy",
  "--copy-metadata",
  "UnityPy",
  "--copy-metadata",
  "Pillow",
  "--copy-metadata",
  "texture2ddecoder",
  "--add-data",
  `${path.join(root, "plugins/asset-tool/src/apmm_util/resources")};apmm_util/resources`,
  "plugins/asset-tool/apmm-util.py",
]);
const payload = path.join(stage, "dist/apmm-asset-tool");
copyFileSync(
  "plugins/asset-tool/THIRD_PARTY_NOTICES.md",
  path.join(payload, "THIRD_PARTY_NOTICES.md"),
);
cpSync(
  "plugins/asset-tool/THIRD_PARTY_LICENSES",
  path.join(payload, "THIRD_PARTY_LICENSES"),
  { recursive: true },
);
const hash = (file) =>
  createHash("sha256").update(readFileSync(file)).digest("hex");
function files(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? files(path.join(dir, e.name)) : [path.join(dir, e.name)],
  );
}
const version = "0.1.0";
const manifest = {
  schema: 1,
  id: "asset-tool",
  version,
  appRange: ">=0.3.37, <1.0.0",
  protocol: 1,
  platform: "windows-x86_64",
  entrypoint: "apmm-asset-tool.exe",
  files: files(payload)
    .map((f) => ({
      path: path.relative(payload, f).replaceAll("\\", "/"),
      bytes: statSync(f).size,
      sha256: hash(f),
    }))
    .sort((a, b) => a.path.localeCompare(b.path)),
};
const raw = JSON.stringify(manifest, null, 2) + "\n";
const privateKey = readFileSync(
  process.env.APMM_PLUGIN_SIGNING_KEY ||
    path.join(work, "signing/asset-tool-local.pem"),
);
const config = JSON.parse(readFileSync("config/auxiliary.json", "utf8"));
const publicKey = createPublicKey(privateKey)
  .export({ format: "der", type: "spki" })
  .subarray(-32)
  .toString("hex");
if (publicKey !== config.publicKey)
  throw Error("签名私钥与主程序配置的公钥不匹配");
const signature = sign(null, Buffer.from(raw), privateKey).toString("hex");
writeFileSync(path.join(payload, "plugin.json"), raw);
writeFileSync(path.join(payload, "plugin.sig"), signature + "\n");
const release = path.join(root, "dist");
mkdirSync(release, { recursive: true });
const zip = path.join(release, `APMM-AssetTool-${version}-windows-x86_64.zip`);
run(["tools/archive.py", payload, zip]);
writeFileSync(
  path.join(release, "asset-tool-release.json"),
  JSON.stringify(
    {
      manifest: raw,
      signature,
      bytes: statSync(zip).size,
      sha256: hash(zip),
      urls: [`${config.baseUrl}/${path.basename(zip)}`],
    },
    null,
    2,
  ) + "\n",
);
writeFileSync(
  path.join(root, "verification-plugin-build.json"),
  JSON.stringify(
    {
      stage,
      payload,
      zip,
      version,
      bytes: statSync(zip).size,
      sha256: hash(zip),
      files: manifest.files,
    },
    null,
    2,
  ),
);
console.log(
  JSON.stringify({ zip, bytes: statSync(zip).size, sha256: hash(zip) }),
);
