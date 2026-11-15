// 对比与紧凑信息编辑在浏览器和正式 WebView 复用相同交互验收。
import assert from "node:assert/strict";
import path from "node:path";
export async function verifyModInfoUI({
  page,
  output,
  checks,
  onRenamed,
  onInfoSaved,
}) {
  await page.setViewportSize({ width: 1147, height: 852 });
  await page.evaluate(() => (document.documentElement.dataset.theme = "light"));
  await page.waitForTimeout(220);
  const dialog = page.getByRole("dialog", { name: "模组信息", exact: true });
  const name = dialog.getByLabel("候选名称", { exact: true });
  assert.ok(!(await name.inputValue()).endsWith(".png"));
  assert.equal(
    await dialog.locator(".mod-file-details").getAttribute("open"),
    null,
  );
  await dialog.locator(".mod-file-details > summary").click();
  assert.equal(
    await dialog.locator(".mod-file-details .resource-code").count(),
    2,
  );
  await dialog.locator(".mod-file-details > summary").click();
  const left = await dialog.locator(".mod-info-previews").boundingBox(),
    right = await dialog.locator(".mod-info-body").boundingBox();
  assert.ok(left && right && right.x > left.x + left.width);
  await name.fill("自由 / 夏日：搭配 🌙");
  await name.press("Enter");
  await dialog.getByText("名称已保存", { exact: true }).waitFor();
  if (onRenamed) await onRenamed("自由 / 夏日：搭配 🌙");
  const authorDetails = dialog.locator(".mod-credit-details").first();
  if (!(await authorDetails.evaluate((e) => e.open)))
    await authorDetails.locator("summary").click();
  for (let i = 1; i <= 2; i++) {
    if (!(await dialog.getByLabel(`作者链接 ${i}`, { exact: true }).count()))
      await dialog
        .getByRole("button", { name: "添加作者链接", exact: true })
        .click();
    await dialog
      .getByLabel(`作者链接 ${i}`, { exact: true })
      .fill(`https://example.com/author${i}`);
  }
  const artistDetails = dialog.locator(".mod-credit-details").nth(1);
  if (!(await artistDetails.evaluate((e) => e.open)))
    await artistDetails.locator("summary").click();
  await dialog.getByLabel("画师", { exact: true }).fill("独立画师");
  for (let i = 1; i <= 2; i++) {
    if (!(await dialog.getByLabel(`画师链接 ${i}`, { exact: true }).count()))
      await dialog
        .getByRole("button", { name: "添加画师链接", exact: true })
        .click();
    await dialog
      .getByLabel(`画师链接 ${i}`, { exact: true })
      .fill(`https://example.com/artist${i}`);
  }
  await dialog.getByRole("button", { name: "保存信息", exact: true }).click();
  if (onInfoSaved) await onInfoSaved();
  const official = dialog.getByRole("region", {
    name: "官方原图 图片预览",
    exact: true,
  });
  const candidate = dialog.getByRole("region", {
    name: "当前候选 图片预览",
    exact: true,
  });
  await candidate.scrollIntoViewIfNeeded();
  const box = await candidate.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.wheel(0, -300);
  await page.waitForTimeout(80);
  assert.notEqual(
    await candidate.locator(".pan-zoom-scale").textContent(),
    "100%",
  );
  assert.equal(await official.locator(".pan-zoom-scale").textContent(), "100%");
  const transform = await candidate
    .locator(".pan-zoom-content")
    .getAttribute("style");
  await page.mouse.down();
  await page.mouse.move(
    box.x + box.width / 2 + 30,
    box.y + box.height / 2 + 20,
  );
  await page.mouse.up();
  assert.notEqual(
    await candidate.locator(".pan-zoom-content").getAttribute("style"),
    transform,
  );
  await candidate.dblclick();
  assert.equal(
    await candidate.locator(".pan-zoom-scale").textContent(),
    "100%",
  );
  await candidate.press("+");
  await dialog
    .getByRole("button", { name: "复位当前候选", exact: true })
    .click();
  assert.equal(
    await candidate.locator(".pan-zoom-scale").textContent(),
    "100%",
  );
  await dialog
    .getByRole("button", { name: "放大当前候选", exact: true })
    .click();
  const zoom = page.getByRole("dialog", { name: "当前候选", exact: true });
  await zoom.waitFor();
  await zoom.getByRole("button", { name: "关闭", exact: true }).click();
  await dialog.locator(".mod-info-body").evaluate((e) => (e.scrollTop = 0));
  await page.screenshot({
    path: path.join(output, "mod-info-comparison-light.png"),
  });
  await page.setViewportSize({ width: 800, height: 880 });
  await page.evaluate(() => (document.documentElement.dataset.theme = "dark"));
  await page.waitForTimeout(220);
  assert.ok(await dialog.evaluate((e) => e.scrollWidth <= e.clientWidth));
  await page.screenshot({
    path: path.join(output, "mod-info-comparison-narrow-dark.png"),
  });
  checks.push(
    "模组信息左侧独立缩放/平移/复位/放大对比，右侧紧凑编辑、折叠文件详情、无后缀 Enter 自由改名和作者/画师多链接",
  );
}
