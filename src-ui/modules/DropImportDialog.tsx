// 仅无法识别且没有明确落点的 PNG 进入归属步骤，已知图片已经独立提交。
import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useActions, useWorkspace } from "../hooks/useApp";
import { IPC } from "../core/ipc/commands";
import { classifyImageRow } from "../core/domain/importClassification";
import { dropSummary, type DropResult } from "../core/domain/dropImport";
import { Modal } from "../shared/Ui";
import { PackageSelection } from "./PackageSelection";

export function DropImportDialog({
  paths,
  hasPackages,
  onClose,
}: {
  paths: string[];
  hasPackages: boolean;
  onClose: () => void;
}) {
  const { api } = useActions();
  const workspace = useWorkspace();
  const client = useQueryClient();
  const [rows, setRows] = useState<any[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [results, setResults] = useState<DropResult[] | null>(null);
  const abort = useRef(new AbortController());
  useEffect(() => {
    abort.current = new AbortController();
    let disposed = false;
    void api
      .command(IPC.images_inspect, { paths })
      .then((preview: any[]) => {
        if (disposed) return;
        setRows(preview);
        setSelected(
          new Set(preview.filter((r) => r.status !== "error").map((r) => r.id)),
        );
      })
      .catch((e) => {
        if (!disposed) setError(String(e));
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
      abort.current.abort();
    };
  }, [api, paths]);
  async function commit() {
    setBusy(true);
    setError("");
    try {
      const chosen = rows.filter((r) => selected.has(r.id));
      const value = await api.importDrop(
        chosen.map((r) => r.path),
        Object.fromEntries(
          chosen.map((r) => [r.path, r.target || { temporary: true }]),
        ),
        abort.current.signal,
        Object.fromEntries(chosen.map((r) => [r.path, r.hash])),
      );
      if (abort.current.signal.aborted) return;
      setResults([
        ...value,
        ...rows
          .filter((r) => r.status === "error")
          .map((r) => ({
            file: r.meta.sourceName,
            ok: false,
            duplicate: false,
            applied: false,
            reason: r.error,
          })),
      ]);
      await client.invalidateQueries({ queryKey: ["workspace"] });
    } catch (e) {
      if (!abort.current.signal.aborted) setError(String(e));
    } finally {
      if (!abort.current.signal.aborted) setBusy(false);
    }
  }
  return (
    <Modal
      open
      wide
      title="选择 PNG 归属"
      onClose={() => {
        if (!busy) onClose();
      }}
    >
      <div className="wizard-body">
        {loading ? (
          <p role="status">正在校验未知图片…</p>
        ) : results ? (
          <div role="status" className="package-result">
            <h2>拖入处理完成</h2>
            <p>{dropSummary(results)}</p>
            {results
              .filter((r) => r.reason || r.applyError)
              .map((r, i) => (
                <p className="danger" key={i}>
                  {r.file}：
                  {r.applyError ? `已入库、未应用：${r.applyError}` : r.reason}
                </p>
              ))}
            <button className="button primary" onClick={onClose}>
              {hasPackages ? "继续导入 APMod" : "返回工作区"}
            </button>
          </div>
        ) : (
          <>
            <p>
              这些图片无法按名称识别，也没有明确部位落点。请选择部位；未选择部位的图片将加入临时资源。
            </p>
            <PackageSelection
              rows={rows}
              selected={selected}
              onSelect={setSelected}
              resources={workspace.data?.resources || []}
              queue=""
              onTarget={(id, row) =>
                setRows((previous) =>
                  previous.map((r) =>
                    r.id === id ? classifyImageRow(r, row) : r,
                  ),
                )
              }
            />
            <p className="muted">
              空部位的首个新 Mod 自动应用；已有候选与临时资源仅加入库。
            </p>
            <div className="dialog-footer">
              <button className="button" disabled={busy} onClick={onClose}>
                {hasPackages ? "跳过图片，继续 APMod" : "取消"}
              </button>
              <button
                className="button primary"
                disabled={busy || !selected.size}
                onClick={() => void commit()}
              >
                {busy ? "正在导入…" : "确认导入"}
              </button>
            </div>
          </>
        )}
        {error && (
          <p className="inline-notice danger" role="alert">
            {error}
          </p>
        )}
      </div>
    </Modal>
  );
}
