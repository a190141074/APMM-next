// 列表与详情共享布局的像素验收，测量真实候选外框与虚拟行边界。
import assert from "node:assert/strict";
// 比较真实边框与图片像素，防止档位名称同步而内部仍使用两套布局。
export async function measureCandidates(page, selector, expectedSize) {
  const row = page.locator(selector).first();
  await row.waitFor();
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
  const metrics = await row.evaluate((element) => {
    const box = (node) => {
      const rect = node.getBoundingClientRect();
      return { width: rect.width, height: rect.height };
    };
    const style = getComputedStyle(element);
    const strip = getComputedStyle(element.querySelector(".candidate-strip"));
    return {
      tiles: [
        ...element.querySelectorAll(
          ".candidate, .candidate-add, .candidate-more",
        ),
      ].map(box),
      images: [...element.querySelectorAll(".candidate .asset-image")].map(box),
      layout: {
        rowGap: style.gap,
        rowPadding: style.padding,
        stripGap: strip.gap,
        stripPadding: strip.padding,
        titleFont: getComputedStyle(element.querySelector("h3")).fontSize,
        buttonPadding: getComputedStyle(
          element.querySelector(".candidate-apply"),
        ).padding,
        addFont: getComputedStyle(element.querySelector(".candidate-add"))
          .fontSize,
      },
    };
  });
  const rows = await page
    .locator(".resource-list-position")
    .evaluateAll((elements) =>
      elements.map((element) => {
        const rect = element.getBoundingClientRect();
        return { top: rect.top, bottom: rect.bottom };
      }),
    );
  assert.ok(
    rows.every(
      (row, index) => index === 0 || row.top >= rows[index - 1].bottom - 1,
    ),
    "虚拟候选行不重叠",
  );
  for (const tile of metrics.tiles) {
    assert.equal(
      tile.width,
      expectedSize,
      `${selector}候选、添加和更多宽度一致`,
    );
    assert.equal(
      tile.height,
      expectedSize,
      `${selector}候选、添加和更多高度一致`,
    );
  }
  for (const image of metrics.images) {
    assert.equal(
      image.width,
      expectedSize - 2,
      `${selector}图片使用边框内完整宽度`,
    );
    assert.equal(image.height, expectedSize - 2, `${selector}图片比例一致`);
  }
  return metrics;
}
