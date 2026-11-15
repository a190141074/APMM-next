// 同路径图片的索引和像素必须一起刷新；验收只改本轮隔离游戏目录。
import assert from "node:assert/strict";
import path from "node:path";
import { statSync, utimesSync, writeFileSync } from "node:fs";
import { png } from "./management-fixtures.mjs";
export async function verifySamePathImageRefresh({
  page,
  game,
  A,
  editedHash,
  row,
  until,
  sha,
  output,
  checks,
}) {
  await page.getByRole("button", { name: "预设", exact: true }).click();
  await page.getByRole("button", { name: "已修复局部", exact: true }).click();
  await page.getByRole("checkbox", { name: /显示全部保存范围/ }).check();
  const item = page.locator(".preset-item").filter({
    has: page.locator("code").filter({ hasText: A.canonical }),
  });
  const current = item.locator('img[alt="当前图片"]');
  const saved = item.locator('img[alt="保存的图片"]');
  await current.waitFor();
  await until(
    async () => (await current.getAttribute("src"))?.includes(editedHash),
    "当前预览旧修订",
  );
  const pixel = (img) =>
    img.evaluate(async (element) => {
      await element.decode();
      // 协议响应允许 CORS，使用当前 img 地址读像素，避免已显示元素的跨域画布污染。
      const readable = new Image();
      readable.crossOrigin = "anonymous";
      readable.src = element.src;
      await readable.decode();
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 1;
      canvas.getContext("2d").drawImage(readable, 0, 0);
      return [...canvas.getContext("2d").getImageData(0, 0, 1, 1).data];
    });
  const before = await pixel(current),
    oldSrc = await current.getAttribute("src"),
    savedSrc = await saved.getAttribute("src");
  const file = path.join(game, A.canonical + ".png"),
    stamp = statSync(file);
  writeFileSync(file, png(73));
  utimesSync(file, stamp.atime, stamp.mtime);
  const hash = sha(file);
  await until(
    async () => (await row(A)).activeHash === hash,
    "同路径修改实际索引",
  );
  await until(
    async () => (await current.getAttribute("src"))?.includes(hash),
    "同路径图片请求修订刷新",
  );
  assert.notEqual(await current.getAttribute("src"), oldSrc);
  assert.notDeepEqual(await pixel(current), before);
  assert.deepEqual(await pixel(current), [73, 30, 40, 255]);
  assert.equal(await saved.getAttribute("src"), savedSrc);
  assert.ok(savedSrc.includes(editedHash));
  const fullSrc = (await current.getAttribute("src"))
    .replace("thumb.localhost", "vault.localhost")
    .replace("thumb://", "vault://");
  assert.equal(
    await page.evaluate(async (url) => {
      const image = new Image();
      image.src = url;
      await image.decode();
      return image.naturalWidth;
    }, fullSrc),
    2,
  );
  await page.screenshot({
    path: path.join(output, "same-path-preview-dark.png"),
  });
  checks.push(
    "同路径同大小且时间戳不变的外部编辑刷新正式 WebView 像素，保存预设图不变，thumb/vault 修订协议均可读取",
  );
}
