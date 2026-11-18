// 一次安装操作串联后台下载、真实终态和收据复核，切换页面不会取消已经授权的更新。
import { appService } from "./app";
import { IPC } from "../ipc/commands";

export type UpdateReceipt = {
  version: string;
  path: string;
  bytes: number;
  kind: string;
};
let installing: Promise<void> | undefined;

/** 只有下载完成且收据版本一致才交给既有安装器；失败和取消均不触发安装。 */
export function installSoftwareUpdate(
  version: string,
  phase: (value: "downloading" | "installing") => void,
  api = appService,
): Promise<void> {
  if (installing) return Promise.reject(new Error("软件更新正在进行中"));
  installing = (async () => {
    let receipt = await api.command<UpdateReceipt | null>(
      IPC.update_download_status,
    );
    if (receipt?.version !== version) {
      phase("downloading");
      const id = await api.task("update-download", "下载并安装软件更新", {
        version,
      });
      const task = await api.waitTask(id);
      if (task.status !== "completed")
        throw new Error(task.message || "更新下载未完成");
      receipt = await api.command<UpdateReceipt | null>(
        IPC.update_download_status,
      );
    }
    if (receipt?.version !== version || !receipt.path)
      throw new Error("更新包与目标版本不一致，请重新检查更新");
    phase("installing");
    await api.command(IPC.update_apply, {
      version,
      assetPath: receipt.path,
    });
  })().finally(() => {
    installing = undefined;
  });
  return installing;
}
