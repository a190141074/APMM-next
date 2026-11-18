import { IPC } from "../core/ipc/commands";
// 首次运行先说明新数据位置，用户确认迁移后才提交后台复制任务。
import { useState } from "react";
import { useActions } from "../hooks/useApp";
import { Modal } from "../shared/Ui";
import { bytes } from "../core/domain/resources";
export function WelcomeDialog({
  dataDir,
  onClose,
}: {
  dataDir: string;
  onClose: () => void;
}) {
  const { api, act, busy, notice } = useActions();
  const [preview, setPreview] = useState<any>(null);
  return (
    <Modal
      open
      title="欢迎使用新版 APMM"
      onClose={() => {
        if (!busy) onClose();
      }}
    >
      <div className="wizard-body">
        <p>可以接续旧版资源库，也可以从空白工作区开始。</p>
        <p className="resource-code">新版数据位置：{dataDir}</p>
        {preview?.mode === "inherited" ? (
          <p>{preview.message}</p>
        ) : preview ? (
          <>
            <p>
              找到 {preview.mods} 个候选（含未应用项）。
              {preview.mode === "merge" ? (
                <>本次补齐 {preview.newMods} 个缺失候选。</>
              ) : (
                <>
                  共 {preview.files} 个文件，约 {bytes(preview.bytes)}。
                </>
              )}
            </p>
            <p className="resource-code">来源：{preview.source}</p>
            <p>
              {preview.mode === "merge"
                ? "保留新版当前应用项、设置和预设，只补齐候选。"
                : "复制并校验成功后启用。"}
              迁移不会自动应用 Mod。
            </p>
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void act(async () => {
                  await api.task(
                    "migration",
                    preview.mode === "merge" ? "补齐旧库候选" : "复制旧资源库",
                    {
                      source: preview.source,
                    },
                  );
                  onClose();
                }, "迁移任务已提交")
              }
            >
              {preview.mode === "merge" ? "补齐缺失候选" : "确认复制"}
            </button>
          </>
        ) : (
          <button
            className="button primary"
            disabled={busy}
            onClick={() =>
              void act(async () => {
                const source = await api.chooseDirectory();
                if (source)
                  setPreview(
                    await api.command(IPC.migration_inspect, { source }),
                  );
              }, "旧库检查完成")
            }
          >
            选择旧版 data 目录
          </button>
        )}
        <button className="button" disabled={busy} onClick={onClose}>
          先进入工作区
        </button>
        {notice && <p className="inline-notice">{notice}</p>}
      </div>
    </Modal>
  );
}
