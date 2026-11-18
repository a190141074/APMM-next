// 只准备 B 盘独立原生验收副本，用于关闭期间的文件变化和损坏缓存场景。
import {
  readFileSync,
  writeFileSync,
  copyFileSync,
  unlinkSync,
  renameSync,
  existsSync,
} from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";

export function prepareRound(root, label) {
  assert.ok(
    path.resolve(root).toLowerCase().startsWith("b:\\apmm-next-work\\"),
  );
  const game = path.join(root, "game/modimg");
  if (label === "offline") {
    const snapshot = JSON.parse(
      readFileSync(path.join(root, "warm-3-snapshot.json"), "utf8"),
    );
    const rows = snapshot.resources
      .filter((r) => r.activeHash && !r.disabled)
      .slice(0, 2);
    const offline = rows.map((row) => ({
      row,
      file: path.join(game, `${row.canonical}.png`),
      bytes: readFileSync(path.join(game, `${row.canonical}.png`)),
    }));
    unlinkSync(offline[0].file);
    copyFileSync(path.join(root, "probe-a.png"), offline[1].file);
    copyFileSync(
      path.join(root, "probe-b.png"),
      path.join(game, "offline-added.png"),
    );
    return offline;
  }
  if (label === "cache-corrupt") {
    for (const name of [
      "directory-verification-v1.json",
      "workspace-snapshot-v1.json",
    ])
      writeFileSync(path.join(root, "app/data/cache", name), "{truncated");
  }
  return undefined;
}

// 暂时断开隔离游戏目录，验证失败观察不会清空候选及应用状态，最后恢复原路径。
export async function probeUnavailableGame(root, invoke, snap, until) {
  assert.ok(
    path.resolve(root).toLowerCase().startsWith("b:\\apmm-next-work\\"),
  );
  const game = path.join(root, "game/modimg");
  const detached = path.join(root, "game/modimg-unavailable");
  assert.ok(!existsSync(detached));
  const previous = (await snap()).resources;
  renameSync(game, detached);
  try {
    await until(async () => (await invoke("directory_sync_status")).lastError);
    assert.deepEqual((await snap()).resources, previous);
  } finally {
    renameSync(detached, game);
  }
  await until(async () => {
    const status = await invoke("directory_sync_status");
    return !status.running && !status.lastError;
  });
  assert.deepEqual((await snap()).resources, previous);
}
