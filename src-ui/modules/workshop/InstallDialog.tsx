import { useState } from "react";
import type { InstallPlan } from "../../core/ipc/generated/InstallPlan";
import { Modal } from "../../shared/Ui";
import { ThemedSelect } from "../../shared/ThemedSelect";
import { workshopService } from "../../core/services/workshop";
import { useActions } from "../../hooks/useApp";
export function InstallDialog({
  plan,
  onClose,
}: {
  plan: InstallPlan;
  onClose: () => void;
}) {
  const [apply, setApply] = useState(false);
  const [selection, setSelection] = useState<Record<string, string>>({});
  const { act, busy, notice } = useActions();
  const slots = [...new Set(plan.entries.map((e) => e.canonical))];
  return (
    <Modal open wide title={`获取 ${plan.work.title}`} onClose={onClose}>
      <div className="wizard-body">
        <label className="check">
          <input
            type="checkbox"
            checked={apply}
            disabled={!plan.canApply}
            onChange={(e) => setApply(e.target.checked)}
          />
          加入并应用到游戏
        </label>
        <p>
          {apply
            ? "确认后会替换下列选中槽位的游戏文件。"
            : "加入本地库，保留当前应用状态。"}
        </p>
        {!plan.canApply && <small>连接游戏目录后可选择应用。</small>}
        {slots.map((slot) => {
          const entries = plan.entries.filter((e) => e.canonical === slot);
          return (
            <section key={slot} className="workshop-slot">
              <strong>{slot}</strong>
              <small>
                当前候选：{entries[0].currentHash?.slice(0, 12) || "未应用"} ·
                游戏文件：{entries[0].gameHashes.length} 个
              </small>
              {entries.map((e) => (
                <div key={e.name}>
                  {e.sourceName}{" "}
                  <small>
                    {e.duplicate ? "库内已存在" : "新增候选"} ·{" "}
                    {e.sha256.slice(0, 12)}
                    {e.disabled ? " · 已禁用" : ""}
                  </small>
                </div>
              ))}
              {apply && (
                <ThemedSelect
                  label={`${slot} 应用候选`}
                  value={selection[slot] || ""}
                  onValueChange={(name) =>
                    setSelection((old) => ({ ...old, [slot]: name }))
                  }
                  options={[
                    { value: "", label: "保留当前应用" },
                    ...entries
                      .filter((e) => !e.disabled)
                      .map((e) => ({
                        value: e.name,
                        label: `${e.sourceName} · ${e.sha256.slice(0, 8)}`,
                      })),
                  ]}
                />
              )}
            </section>
          );
        })}
        {notice && <p role="status">{notice}</p>}
      </div>
      <footer className="wizard-footer">
        <button className="button" onClick={onClose}>
          取消
        </button>
        <button
          className="button primary"
          disabled={busy}
          onClick={() =>
            void act(async () => {
              await workshopService.install({
                id: plan.work.id,
                workFingerprint: plan.workFingerprint,
                localFingerprint: plan.localFingerprint,
                applyFiles: apply
                  ? Object.values(selection).filter(Boolean)
                  : [],
              });
              onClose();
            }, "任务已加入后台队列")
          }
        >
          {apply ? "确认加入并应用" : "加入本地库"}
        </button>
      </footer>
    </Modal>
  );
}
