// 创建预设时选择固定槽位，角色默认选中所有现有变体和部位。
import { useState } from "react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import { useActions } from "../hooks/useApp";
import { usePresetService } from "../hooks/useManagement";
import { Modal } from "../shared/Ui";
import { ThemedSelect } from "../shared/ThemedSelect";
import { slotOfResource } from "../core/domain/resources";
export function CreatePresetDialog({
  resources: rows,
  onClose,
  onSaved,
}: {
  resources: ResourceRecord[];
  onClose: () => void;
  onSaved: (id: string) => void;
}) {
  const { act, busy, notice } = useActions();
  const service = usePresetService();
  const [name, setName] = useState("");
  const [mode, setMode] = useState("global");
  const [entity, setEntity] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [query, setQuery] = useState("");
  const characters = [
    ...new Map(
      rows
        .filter((r) => r.category === "heroes")
        .map((r) => [r.entityId, { value: r.entityId, label: r.entityName }]),
    ).values(),
  ];
  const available = rows.filter(
    (r) =>
      (mode !== "character" || r.entityId === entity) &&
      `${r.entityName} ${r.variantLabel} ${r.resourceLabel} ${r.canonical}`
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  const chooseEntity = (next: string) => {
    setEntity(next);
    setSelected(
      new Set(
        rows
          .filter((r) => r.entityId === next && r.category === "heroes")
          .map((r) => r.key),
      ),
    );
  };
  return (
    <Modal open wide onClose={() => !busy && onClose()} title="创建预设">
      <div className="wizard-body">
        <label className="field">
          预设名称
          <input value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <ThemedSelect
          label="创建范围"
          value={mode}
          onValueChange={(v) => {
            setMode(v);
            setSelected(new Set());
            if (v === "character" && characters[0])
              chooseEntity(characters[0].value);
          }}
          options={[
            { value: "global", label: "全局搭配" },
            { value: "character", label: "指定角色" },
            { value: "parts", label: "选定部位" },
          ]}
        />
        {mode !== "global" && (
          <>
            {mode === "character" && (
              <ThemedSelect
                label="预设角色"
                value={entity}
                options={characters}
                onValueChange={chooseEntity}
              />
            )}
            <p>角色默认包含当前全部变体和部位，可取消选择。保存后范围固定。</p>
            <input
              aria-label="筛选预设槽位"
              placeholder="筛选对象、变体或部位"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <p>已选择 {selected.size} 个槽位</p>
            <div className="action-row">
              <button
                className="button"
                onClick={() =>
                  setSelected(
                    new Set([...selected, ...available.map((r) => r.key)]),
                  )
                }
              >
                选择筛选结果
              </button>
              <button className="button" onClick={() => setSelected(new Set())}>
                清空选择
              </button>
            </div>
            <div className="scope-options">
              {available.map((r) => (
                <label key={r.key}>
                  <input
                    type="checkbox"
                    checked={selected.has(r.key)}
                    onChange={(e) =>
                      setSelected((old) => {
                        const n = new Set(old);
                        e.target.checked ? n.add(r.key) : n.delete(r.key);
                        return n;
                      })
                    }
                  />
                  {r.entityName} · {r.variantLabel} ·{" "}
                  {r.resourceLabel || r.sectionLabel}
                  <code>{r.canonical}</code>
                </label>
              ))}
            </div>
          </>
        )}
        <p>保存有效搭配；原版和禁用部位都记录为原版。</p>
        <button
          className="button primary"
          disabled={
            busy || !name.trim() || (mode !== "global" && !selected.size)
          }
          onClick={() =>
            void act(async () => {
              const p = await service.create(
                name,
                mode === "global" ? "global" : "local",
                rows.filter((r) => selected.has(r.key)).map(slotOfResource),
              );
              onSaved(p.id);
            })
          }
        >
          保存当前搭配
        </button>
        {notice && <p role="status">{notice}</p>}
      </div>
    </Modal>
  );
}
