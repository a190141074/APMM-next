// 旧库发现是只读查询；同目录接续不再要求选择自己，独立安装的缺失候选持续提示。
import { useQuery } from "@tanstack/react-query";
import { IPC } from "../core/ipc/commands";
import { appService } from "../core/services/app";
import type { MigrationStatus } from "../core/ipc/generated/MigrationStatus";

export function useMigrationStatus(revision: number) {
  return useQuery({
    queryKey: [IPC.migration_status, { revision }],
    queryFn: () => appService.command<MigrationStatus>(IPC.migration_status),
    enabled: appService.native,
    staleTime: 15000,
  });
}

export function LegacyMigrationNotice({
  revision,
  onReview,
}: {
  revision: number;
  onReview: () => void;
}) {
  const status = useMigrationStatus(revision);
  const missing = status.data?.sources.find((source) => source.newMods > 0);
  if (!missing) return null;
  return (
    <div className="migration-reminder" role="status">
      <span>
        检测到旧库，仍有 {missing.newMods} 个候选可补齐，包含未应用的备选。
      </span>
      <button className="text-button" onClick={onReview}>
        检查旧库备选
      </button>
    </div>
  );
}
