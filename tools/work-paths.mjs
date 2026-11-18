// 发行产物和验收报告使用独立工作盘，版本目录不混放不同轮次。
import { readFileSync, mkdirSync } from "node:fs";
import path from "node:path";

export const root = path.resolve(import.meta.dirname, "..");
export const version = readFileSync(path.join(root, "VERSION"), "utf8").trim();
export const work = process.env.APMM_WORK_DIR || "B:/APMM-next-work";
export function outsideSource(value) {
  const resolved = path.resolve(value);
  const relative = path.relative(root, resolved);
  if (
    !relative ||
    (!relative.startsWith(".." + path.sep) && !path.isAbsolute(relative))
  )
    throw Error("发行产物和报告必须保存到源码目录之外");
  return resolved;
}
export const reportDir = outsideSource(
  process.env.APMM_REPORT_DIR || path.join(work, "reports", version),
);
export const releaseDir = outsideSource(
  process.env.APMM_DIST_DIR || path.join(work, "releases", version),
);
export const portableBuildReport =
  process.env.APMM_BUILD_REPORT || path.join(reportDir, "portable-build.json");
export const distributionsReport =
  process.env.APMM_DISTRIBUTIONS_REPORT ||
  path.join(reportDir, "distributions.json");
export function prepareParent(file) {
  const resolved = outsideSource(file);
  mkdirSync(path.dirname(resolved), { recursive: true });
  return resolved;
}
