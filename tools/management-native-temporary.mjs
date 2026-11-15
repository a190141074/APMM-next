// 正式目录监听对临时 PNG 的外部启禁和本机选择分别验收，文件仅在 B 盘副本中。
import { writeFileSync, renameSync } from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { png } from "./management-fixtures.mjs";
export async function verifyNativeTemporary({
  page,
  game,
  call,
  snapshot,
  until,
  task,
  sha,
  checks,
}) {
  const enabled = path.join(game, "unrecognized-native.png"),
    parked = enabled + ".disabled";
  writeFileSync(parked, png(77));
  const hash = sha(parked);
  const row = async () =>
    (await snapshot()).resources.find(
      (r) => r.category === "temporary" && r.files.some((f) => f.hash === hash),
    );
  await until(async () => (await row())?.disabled === true, "外部禁用临时新增");
  const file = (await row()).files.find((f) => f.hash === hash);
  assert.ok(file.path.endsWith(".disabled"));
  assert.equal(
    await page.evaluate(
      async ({ path, hash }) => {
        const image = new Image();
        image.src =
          window.__TAURI_INTERNALS__.convertFileSrc(path, "vault") +
          "?revision=" +
          hash;
        await image.decode();
        return image.naturalWidth;
      },
      { path: file.path, hash },
    ),
    2,
  );
  renameSync(parked, enabled);
  await until(async () => (await row())?.disabled === false, "外部临时启用");
  const r = await row(),
    args = {
      heroId: r.entityId,
      sectionKey: r.section,
      canonical: r.canonical,
      hash,
    };
  await call("mod_set_disabled", { args, disabled: true });
  await task("scan", {});
  assert.equal((await row()).disabled, true);
  assert.ok(
    (await row()).files.find((f) => f.hash === hash).path.endsWith(".disabled"),
  );
  await call("mod_set_disabled", { args, disabled: false });
  renameSync(enabled, parked);
  await until(async () => (await row())?.disabled === true, "外部临时再次禁用");
  renameSync(parked, enabled);
  await until(
    async () => (await row())?.disabled === false,
    "外部临时再次启用",
  );
  checks.push(
    "正式外部临时 PNG disabled 后缀同步、禁用图片只读预览及重复扫描保留本机启禁选择",
  );
}
