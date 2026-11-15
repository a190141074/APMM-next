// 关于页验收复用于浏览器和正式 WebView，核对半宽布局、链接及窄窗口堆叠。
import assert from "node:assert/strict";
import path from "node:path";
export async function verifyManagementAbout({ page, output, checks }) {
  await page.getByRole("button", { name: "关于", exact: true }).click();
  await page.setViewportSize({ width: 1122, height: 743 });
  await page.evaluate(() => (document.documentElement.dataset.theme = "light"));
  await page.waitForTimeout(220);
  const reference = page.locator(".settings-card").filter({
    has: page.getByRole("heading", { name: "鸣谢 · 参考项目", exact: true }),
  });
  const tools = page.locator(".settings-card").filter({
    has: page.getByRole("heading", { name: "鸣谢 · 工具", exact: true }),
  });
  await reference.waitFor();
  const a = await reference.boundingBox(),
    b = await tools.boundingBox();
  assert.ok(a && b && Math.abs(a.y - b.y) < 1 && b.x > a.x + a.width);
  assert.ok(Math.abs(a.width - b.width) < 1);
  const lower = await page
    .locator(".about-credit-groups")
    .nth(1)
    .locator(".settings-card")
    .first()
    .boundingBox();
  assert.ok(lower && Math.abs(lower.width - a.width) < 1);
  assert.equal(await reference.getByText("APX", { exact: true }).count(), 0);
  assert.equal(
    await reference
      .locator('a[href="https://github.com/Jorixon/JASM"]')
      .count(),
    1,
  );
  assert.equal(
    await tools
      .locator('a[href="https://cnb.cool/erinilis/apx-plugin-releases"]')
      .count(),
    1,
  );
  assert.equal(
    await tools.locator('a[href="https://openai.com/codex/"]').count(),
    1,
  );
  await page.screenshot({ path: path.join(output, "about-1122-light.png") });
  await page.setViewportSize({ width: 800, height: 880 });
  await page.evaluate(() => (document.documentElement.dataset.theme = "dark"));
  await page.waitForTimeout(220);
  const narrowA = await reference.boundingBox(),
    narrowB = await tools.boundingBox();
  assert.ok(
    narrowA &&
      narrowB &&
      Math.abs(narrowA.x - narrowB.x) < 1 &&
      narrowB.y > narrowA.y + narrowA.height,
  );
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({ path: path.join(output, "about-narrow-dark.png") });
  checks.push(
    "关于页参考项目与工具同宽并排，JASM/APX/Codex 链接，窄窗口深色堆叠",
  );
}
