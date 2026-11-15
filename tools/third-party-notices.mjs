// 直接从已解析依赖和本地许可证生成归属说明，不推测依赖许可证内容。
import { readFileSync, readdirSync, existsSync, writeFileSync } from "node:fs";
import path from "node:path";
const root = path.resolve(import.meta.dirname, "..");
const metadata = JSON.parse(
  readFileSync(path.join(root, "verification-cargo-metadata.json"), "utf8"),
);
const entries = new Map();
function add(name, version, license, directory) {
  if (entries.has(name + "@" + version)) return;
  const texts = readdirSync(directory, { withFileTypes: true })
    .filter(
      (e) => e.isFile() && /^(licen[sc]e|copying|notice)([.-]|$)/i.test(e.name),
    )
    .map(
      (e) =>
        `### ${e.name}\n\n${readFileSync(path.join(directory, e.name), "utf8")}`,
    );
  entries.set(
    name + "@" + version,
    `## ${name} ${version}\n\n声明许可：${typeof license === "string" ? license : JSON.stringify(license) || "见源码许可"}\n\n${texts.join("\n\n")}`,
  );
}
for (const p of metadata.packages)
  if (p.name !== "astral-mod-manager")
    add(p.name, p.version, p.license, path.dirname(p.manifest_path));
const pnpm = path.join(root, "node_modules/.pnpm");
for (const group of readdirSync(pnpm, { withFileTypes: true }).filter((e) =>
  e.isDirectory(),
)) {
  const modules = path.join(pnpm, group.name, "node_modules");
  if (!existsSync(modules)) continue;
  for (const entry of readdirSync(modules, { withFileTypes: true }).filter(
    (e) => e.isDirectory(),
  )) {
    const dirs = entry.name.startsWith("@")
      ? readdirSync(path.join(modules, entry.name)).map((n) =>
          path.join(modules, entry.name, n),
        )
      : [path.join(modules, entry.name)];
    for (const dir of dirs) {
      const file = path.join(dir, "package.json");
      if (!existsSync(file)) continue;
      const p = JSON.parse(readFileSync(file, "utf8"));
      add(p.name, p.version, p.license, dir);
    }
  }
}
add("WiX Toolset", "3.14.1", "Microsoft Reciprocal License (MS-RL)", path.join(root, "vendor/wix"));
writeFileSync(
  path.join(root, "THIRD_PARTY_NOTICES.md"),
  "# APMM 第三方归属说明\n\n本文件从锁定依赖及其本地许可证生成。NapCatQQ-Desktop 仅作为布局与交互参考，本工程未复制其源码。历史兼容模块、图标和吸猫动画迁自旧 APMM。WebView2Loader.dll 使用旧版已分发的 Microsoft WebView2 运行库加载器。\n\n" +
    [...entries.values()].join("\n\n---\n\n"),
);
console.log(`已记录 ${entries.size} 个构建与运行依赖。`);
