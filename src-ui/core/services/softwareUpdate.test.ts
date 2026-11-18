import { describe, expect, it, vi } from "vitest";
vi.mock("./app", () => ({ appService: {} }));
import { installSoftwareUpdate } from "./softwareUpdate";
import { appService } from "./app";

function fixture(
  cached = false,
  status = "completed",
  receiptVersion = "0.4.12",
) {
  let downloaded = cached;
  const command = vi.fn(async (name: string) => {
    if (name === "update_download_status")
      return downloaded
        ? { version: receiptVersion, path: "B:/update.zip" }
        : null;
  });
  const task = vi.fn(async () => "download-1");
  const waitTask = vi.fn(async () => {
    downloaded = true;
    return { status, message: "下载中止" };
  });
  return {
    command,
    task,
    waitTask,
    api: { command, task, waitTask } as unknown as typeof appService,
  };
}

describe("一次点击安装软件更新", () => {
  it("等待下载真实完成、重新读取收据后才安装", async () => {
    const f = fixture();
    const phase = vi.fn();
    await installSoftwareUpdate("0.4.12", phase, f.api);
    expect(f.task).toHaveBeenCalledWith(
      "update-download",
      "下载并安装软件更新",
      { version: "0.4.12" },
    );
    expect(f.command.mock.calls.map(([name]) => name)).toEqual([
      "update_download_status",
      "update_download_status",
      "update_apply",
    ]);
    expect(f.command).toHaveBeenLastCalledWith("update_apply", {
      version: "0.4.12",
      assetPath: "B:/update.zip",
    });
    expect(phase.mock.calls.flat()).toEqual(["downloading", "installing"]);
  });
  it("同版本已下载时复用收据直接安装", async () => {
    const f = fixture(true);
    await installSoftwareUpdate("0.4.12", vi.fn(), f.api);
    expect(f.task).not.toHaveBeenCalled();
    expect(f.command).toHaveBeenLastCalledWith(
      "update_apply",
      expect.anything(),
    );
  });
  it.each(["failed", "cancelled", "interrupted"])(
    "下载 %s 时不安装",
    async (status) => {
      const f = fixture(false, status);
      await expect(
        installSoftwareUpdate("0.4.12", vi.fn(), f.api),
      ).rejects.toThrow("下载中止");
      expect(
        f.command.mock.calls.some(([name]) => name === "update_apply"),
      ).toBe(false);
    },
  );
  it("下载返回错误版本收据时不安装", async () => {
    const f = fixture(false, "completed", "0.4.11");
    await expect(
      installSoftwareUpdate("0.4.12", vi.fn(), f.api),
    ).rejects.toThrow("目标版本不一致");
    expect(f.command.mock.calls.some(([name]) => name === "update_apply")).toBe(
      false,
    );
  });
  it("拒绝重复提交且失败后允许重试", async () => {
    const f = fixture();
    const first = installSoftwareUpdate("0.4.12", vi.fn(), f.api);
    await expect(
      installSoftwareUpdate("0.4.12", vi.fn(), f.api),
    ).rejects.toThrow("正在进行中");
    await first;
    f.command.mockRejectedValueOnce(new Error("网络失败"));
    await expect(
      installSoftwareUpdate("0.4.12", vi.fn(), f.api),
    ).rejects.toThrow("网络失败");
    await installSoftwareUpdate("0.4.12", vi.fn(), f.api);
  });
});
