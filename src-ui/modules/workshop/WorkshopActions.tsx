// 卡片和详情共用获取入口，均交给既有预检及安装确认流程。
import type { ListedWork } from "../../core/ipc/generated/ListedWork";
import { workStatus } from "../../core/domain/workshop";

export function WorkshopActions({
  item,
  busy,
  onAcquire,
}: {
  item: ListedWork;
  busy: boolean;
  onAcquire: () => void;
}) {
  const state = workStatus(item);
  return (
    <button
      type="button"
      className="button primary"
      disabled={busy || state === "installed"}
      onClick={onAcquire}
    >
      {state === "installed"
        ? "已是当前版本"
        : state === "update"
          ? "检查更新内容"
          : "获取作品"}
    </button>
  );
}
