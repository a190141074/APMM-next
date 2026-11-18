// 设置相关批注在隔离传输下验收，不执行真实数据迁移或安装。
import assert from "node:assert/strict";
import path from "node:path";
import { writeFileSync } from "node:fs";

export async function verifyInheritanceDetection(page, output, go) {
  await page.evaluate(() => {
    localStorage.removeItem("six.fresh");
    localStorage.removeItem("six.detected");
    localStorage.setItem("six.inherited", "true");
  });
  await page.reload();
  await go("设置", "settings");
  await page.getByText(/已就地接续当前 data/).waitFor();
  assert.equal(
    await page.getByRole("button", { name: /选择.*data 目录/ }).count(),
    0,
    "当前 data 已继承时不再显示旧 data 选择入口",
  );
  assert.equal(
    await page.evaluate(
      () => window.__six.calls.filter((c) => c.name === "migration").length,
    ),
    0,
  );
  await page.screenshot({ path: path.join(output, "10-inherited-data.png") });
  await page.evaluate(() => {
    localStorage.removeItem("six.inherited");
    localStorage.setItem("six.detected", "true");
  });
  await page.reload();
  await page.getByRole("button", { name: "检查旧库备选", exact: true }).click();
  await page.getByRole("button", { name: "检查并补齐", exact: true }).click();
  const merge = page.getByRole("dialog", {
    name: "确认补齐旧库候选",
    exact: true,
  });
  await merge.waitFor();
  assert.match(await merge.innerText(), /226 个缺失候选/);
  await page.screenshot({ path: path.join(output, "11-detected-backups.png") });
  await merge.getByRole("button", { name: "关闭", exact: true }).click();
}

export async function verifyMigrationDialog(page, output) {
  await page.evaluate(() => localStorage.setItem("six.fresh", "true"));
  await page.evaluate(() => {
    window.__six.inherited = false;
    window.__six.detected = false;
    window.__six.fresh = true;
  });
  await page.reload();
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.locator('main[data-route="settings"]').waitFor();
  await page
    .getByRole("button", { name: "手动选择旧 data 目录", exact: true })
    .click();
  const migration = page.getByRole("dialog", {
    name: "确认补齐旧库候选",
    exact: true,
  });
  await migration.waitFor();
  assert.match(await migration.innerText(), /1,?028 个候选（含未应用项）/);
  assert.match(await migration.innerText(), /226 个缺失候选/);
  assert.match(await migration.innerText(), /保留新版当前应用项、设置和预设/);
  assert.equal(
    await page.evaluate(
      () => window.__six.calls.filter((c) => c.name === "migration").length,
    ),
    0,
  );
  await page.screenshot({ path: path.join(output, "08-migration-merge.png") });
  await migration
    .getByRole("button", { name: "补齐缺失候选", exact: true })
    .click();
  await migration.waitFor({ state: "detached" });
  assert.equal(
    await page.evaluate(
      () => window.__six.calls.filter((c) => c.name === "migration").length,
    ),
    1,
  );
  await page.evaluate(() => {
    window.__six.migrationMode = "copy";
  });
  await page
    .getByRole("button", { name: "手动选择旧 data 目录", exact: true })
    .click();
  const copy = page.getByRole("dialog", {
    name: "确认复制旧资源库",
    exact: true,
  });
  await copy.getByRole("button", { name: "复制并迁移", exact: true }).waitFor();
  await copy.getByRole("button", { name: "关闭", exact: true }).click();
}

export async function verifyMinuteSchedule(page, output, go) {
  await go("设置", "settings");
  await page.getByRole("tab", { name: "更新", exact: true }).click();
  const interval = page.getByRole("spinbutton", {
    name: "检查间隔（分钟）",
    exact: true,
  });
  assert.equal(await interval.inputValue(), "60");
  // 将最近检查时间设为当前时刻，观察真实 hook 的定时触发与关闭后的取消。
  await page.clock.install();
  await page.evaluate(async () => {
    const { demo } = await import("/src-ui/core/ipc/mock.ts");
    demo.settings.updateLastCheckedAt = Date.now() / 1000;
  });
  await interval.fill("1");
  await interval.blur();
  await page.waitForFunction(
    async () =>
      (await import("/src-ui/core/ipc/mock.ts")).demo.settings
        .updateIntervalMinutes === 1,
  );
  await go("总览", "overview");
  await page.evaluate(() => {
    window.__six.calls = [];
  });
  await page.clock.fastForward(59000);
  writeFileSync(
    path.join(output, "minute-before-due.json"),
    JSON.stringify(await page.evaluate(() => window.__six.calls)),
  );
  assert.equal(
    await page.evaluate(
      () => window.__six.calls.filter((c) => c.name === "update_check").length,
    ),
    0,
    "未满一分钟不应自动检查",
  );
  await page.clock.fastForward(2000);
  await page.waitForFunction(() =>
    window.__six.calls.some((c) => c.name === "update_check"),
  );
  await go("设置", "settings");
  await page.getByRole("tab", { name: "更新", exact: true }).click();
  assert.equal(await interval.inputValue(), "1");
  await page.screenshot({ path: path.join(output, "09-update-minutes.png") });
  await page.getByRole("checkbox", { name: /自动检查更新/ }).uncheck();
  await page.waitForFunction(
    async () =>
      (await import("/src-ui/core/ipc/mock.ts")).demo.settings
        .updateAutoCheck === false,
  );
  await page.waitForTimeout(250);
  await page.clock.runFor(10);
  await page.evaluate(() => {
    window.__six.calls = [];
  });
  await page.clock.fastForward(120000);
  writeFileSync(
    path.join(output, "minute-after-disabled.json"),
    JSON.stringify(await page.evaluate(() => window.__six.calls)),
  );
  assert.equal(
    await page.evaluate(
      () => window.__six.calls.filter((c) => c.name === "update_check").length,
    ),
    0,
    "关闭自动检查后不应继续调度",
  );
}
