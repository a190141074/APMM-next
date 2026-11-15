// IPC 命令从 Rust 注册面生成；界面只引用集中常量。
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import path from "node:path";
const root = path.resolve(import.meta.dirname, "..");
const source = readFileSync(path.join(root, "src-tauri/src/lib.rs"), "utf8");
const block = source.split("tauri::generate_handler![")[1]?.split("]")[0];
if (!block) throw Error("未找到命令注册面");
const names = [
  ...new Set(
    block
      .split(",")
      .map((n) => n.trim().split("::").pop())
      .filter(Boolean),
  ),
].sort();
const target = path.join(root, "src-ui/core/ipc/commands.ts");
writeFileSync(
  target,
  "// 由 tools/sync-ipc.mjs 从 Rust 命令注册面生成。\nexport const IPC = " +
    JSON.stringify(Object.fromEntries(names.map((n) => [n, n])), null, 2) +
    ' as const;\nexport const EVENTS = { taskProgress: "task-progress", workspaceChanged: "workspace-changed" } as const;\n',
);
if (process.argv.includes("--migrate")) {
  const walk = (dir) =>
    readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
      e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)],
    );
  for (const file of walk(path.join(root, "src-ui"))) {
    if (
      !/\.tsx?$/.test(file) ||
      file === target ||
      file.includes(`${path.sep}generated${path.sep}`)
    )
      continue;
    const original = readFileSync(file, "utf8");
    let changed = original.replace(/"([a-z_]+)"/g, (match, name) =>
      names.includes(name) ? `IPC.${name}` : match,
    );
    if (changed !== original) {
      let relative = path
        .relative(path.dirname(file), target)
        .replaceAll("\\", "/")
        .replace(/\.ts$/, "");
      if (!relative.startsWith(".")) relative = "./" + relative;
      if (!/import\s*\{\s*IPC\s*\}/.test(changed))
        changed = `import {IPC} from "${relative}";\n` + changed;
      writeFileSync(file, changed);
    }
  }
}
