// 缓存尚未下载时询问用户；提醒开关保存在工作区设置中。
import { Download } from "lucide-react";
import { useEffect, useState } from "react";
import { useActions, usePreviewStatus } from "../hooks/useApp";
import { Modal } from "../shared/Ui";
export function PreviewDownloadPrompt({ enabled }: { enabled: boolean }) {
  const status = usePreviewStatus(enabled);
  const { api, act } = useActions();
  const [dismissed, setDismissed] = useState(false);
  useEffect(() => {
    if (enabled) setDismissed(false);
  }, [enabled]);
  const missingFiles = status.data?.missingFiles ?? 0;
  const missingBundle = status.data?.bundles?.some(
    (bundle) => !bundle.localVersion,
  );
  if (!enabled || dismissed || !status.isSuccess || !missingBundle) return null;
  const close = () => setDismissed(true);
  return (
    <Modal open title="尚未下载预览图" onClose={close}>
      <div className="wizard-body preview-download-prompt">
        <p>检测到本地尚未下载预览资源（{missingFiles} 张），是否现在下载？</p>
        <p className="muted">
          选择“不再提醒”后，可在设置 → 目录与资源中重新开启提醒，也可随时手动补下载。
        </p>
        <div className="dialog-footer">
          <button
            className="button primary"
            onClick={() => {
              close();
              void act(() => api.task("preview", "下载预览图", {}));
            }}
          >
            <Download size={15} />
            是
          </button>
          <button className="button" onClick={close}>
            否
          </button>
          <button
            className="button subtle"
            onClick={() => {
              close();
              void act(
                () => api.settings({ previewDownloadPrompt: false }),
                "已关闭预览图提醒。可在设置 → 目录与资源中重新开启提醒，或手动下载预览图。",
              );
            }}
          >
            不再提醒
          </button>
        </div>
      </div>
    </Modal>
  );
}
