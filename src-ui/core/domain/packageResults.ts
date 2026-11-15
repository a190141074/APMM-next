// 统一解释图片逐项结果、包导入计数与导出分卷，结果只来自后台终态。
import type { TaskSnapshot } from "../ipc/generated/TaskSnapshot";
export type Summary = {
  success: number;
  skipped: number;
  failed: number;
  paths: string[];
  errors: string[];
};
export function addResult(
  total: Summary,
  task: TaskSnapshot,
  expected: number,
) {
  if (task.status !== "completed") {
    total.failed += expected;
    total.errors.push(task.message);
    return;
  }
  const result: any = task.result;
  if (Array.isArray(result)) {
    for (const r of result) {
      if (!r.ok) {
        total.failed++;
        total.errors.push(`${r.file}：${r.reason || "导入失败"}`);
      } else if (r.duplicate) total.skipped++;
      else total.success++;
    }
  } else if (typeof result === "number") {
    total.success += result;
    total.skipped += Math.max(0, expected - result);
  } else if (result && typeof result === "object") {
    total.success += result.files || 0;
    total.paths.push(
      ...(result.volumes?.length ? result.volumes : [result.path]).filter(
        Boolean,
      ),
    );
  }
}
