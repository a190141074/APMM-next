// 同步状态独立于资源查询；进度通知不会让整个列表重新加载。
import { useDirectorySync } from "../hooks/useApp";

export function DirectorySyncIndicator() {
  const { data } = useDirectorySync();
  if (!data || (!data.running && !data.lastError)) return null;
  return (
    <small
      role="status"
      title={data.lastError ?? "先显示已有记录，核对完成后自动更新。"}
    >
      {data.running
        ? `${data.phase === "full" ? "完整校验" : "后台核对"}${data.total ? ` ${data.checked}/${data.total}` : "中…"}`
        : "目录核对未完成"}
    </small>
  );
}
