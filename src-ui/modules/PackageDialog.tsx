import { classifyImageRow } from "../core/domain/importClassification";
// 导入导出向导管理临时队列；任务完成后才展示真实统计，关闭时释放未提交队列。
import { useState, useEffect, useRef } from "react";
import {
  FolderOpen,
  Check,
  ArrowRight,
  FileImage,
  LockKeyhole,
} from "lucide-react";
import { IPC } from "../core/ipc/commands";
import { ThemedSelect } from "../shared/ThemedSelect";
import { useActions, useWorkspace } from "../hooks/useApp";
import { Modal } from "../shared/Ui";
import { uniquePackages, type ImportTarget } from "../core/domain/packages";
import { PackageSelection } from "./PackageSelection";
import { addResult, type Summary } from "../core/domain/packageResults";

export function PackageDialog({
  mode,
  onClose,
  initialPaths = [],
  target,
  importKind = "all",
}: {
  mode: "import" | "export";
  onClose: () => void;
  initialPaths?: string[];
  target?: ImportTarget;
  importKind?: "images" | "packages" | "all";
}) {
  const { api, act, busy, notice } = useActions();
  const workspace = useWorkspace();
  const [step, setStep] = useState(0);
  const [paths, setPaths] = useState(initialPaths);
  const [password, setPassword] = useState("");
  const [passwords, setPasswords] = useState<Record<string, string>>({});
  const queue = useRef("");
  const waiting = useRef(false);
  const abort = useRef(new AbortController());
  const [rows, setRows] = useState<any[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [apply, setApply] = useState(false);
  const [format, setFormat] = useState("apmod");
  const [title, setTitle] = useState("APMM-Mods");
  const [taskId, setTaskId] = useState("");
  const [summary, setSummary] = useState<Summary>({
    success: 0,
    skipped: 0,
    failed: 0,
    paths: [],
    errors: [],
  });
  const [processing, setProcessing] = useState(false);
  useEffect(() => {
    abort.current = new AbortController();
    void api
      .command(IPC.credential_default_password_get)
      .then((p) => {
        if (typeof p === "string") setPassword(p);
      })
      .catch(() => {});
    return () => {
      abort.current.abort();
      if (queue.current && !waiting.current)
        void api
          .command(IPC.package_import_queue_cancel, {
            args: { queueId: queue.current },
          })
          .catch(() => {});
    };
  }, [api]);
  async function inspect() {
    await act(async () => {
      if (queue.current) {
        await api.command(IPC.package_import_queue_cancel, {
          args: { queueId: queue.current },
        });
        queue.current = "";
      }
      let previews: any[] = [];
      if (mode === "export") {
        const candidates = await api.command(IPC.package_export_candidates);
        previews = (
          Array.isArray(candidates) ? candidates : candidates.files || []
        ).map((x: any, i: number) => ({ ...x, id: String(i) }));
      } else {
        const packages = uniquePackages(paths);
        const images = paths.filter((p) => p.toLowerCase().endsWith(".png"));
        if (packages.length) {
          const begin = await api.command(IPC.package_import_queue_begin);
          queue.current = begin.queueId;
          try {
            for (const path of packages)
              await api.command(IPC.package_import_queue_add, {
                args: {
                  queueId: begin.queueId,
                  path,
                  password: passwords[path] || password,
                },
              });
            previews = (
              await api.command(IPC.package_import_queue_preview, {
                args: { queueId: begin.queueId },
              })
            ).files;
          } catch (e) {
            await api.command(IPC.package_import_queue_cancel, {
              args: { queueId: begin.queueId },
            });
            queue.current = "";
            throw e;
          }
        }
        if (images.length)
          previews.push(
            ...(await api.command(IPC.images_inspect, {
              paths: images,
              target,
            })),
          );
      }
      setRows(previews);
      setSelected(
        new Set(
          previews
            .filter((x) => x.status !== "duplicate" && x.status !== "error")
            .map((x) => x.id),
        ),
      );
      setStep(1);
    }, "识别完成");
  }
  async function runTask(kind: string, args: Record<string, unknown>) {
    const id = await api.task(
      kind,
      mode === "export" ? "导出 Mod 包" : "导入资源",
      args,
    );
    setTaskId(id);
    return api.waitTask(id, abort.current.signal);
  }
  async function commit() {
    let destination: string | null = null;
    if (mode === "export") {
      destination = await api.saveFile(
        title + (format === "apmod" ? ".apmod" : ".zip"),
      );
      if (!destination) return;
    }
    setProcessing(true);
    setStep(2);
    waiting.current = true;
    const total: Summary = {
      success: 0,
      skipped: rows.filter((x) => !selected.has(x.id) && x.status !== "error")
        .length,
      failed: rows.filter((x) => x.status === "error").length,
      paths: [],
      errors: rows.filter((x) => x.status === "error").map((x) => x.error),
    };
    await act(async () => {
      try {
        if (mode === "export") {
          const items = rows
            .filter((x) => selected.has(x.id))
            .map((x) => ({
              category: x.category,
              entityId: x.entityId,
              sectionKey: x.sectionKey,
              canonical: x.canonical,
              hash: x.hash,
            }));
          addResult(
            total,
            await runTask(
              format === "apmod" ? "package-export" : "quick-export",
              {
                destination,
                password,
                items,
                mode: "zip",
                includeApplicationState: apply,
                maxVolumeBytes: 512 * 1024 * 1024,
              },
            ),
            items.length,
          );
        } else {
          const images = rows.filter((x) => selected.has(x.id) && x.path);
          if (images.length)
            addResult(
              total,
              await runTask("images-import", {
                paths: images.map((x) => x.path),
                expectedHashes: Object.fromEntries(
                  images.map((x) => [x.path, x.hash]),
                ),
                target,
                targets: Object.fromEntries(
                  images
                    .filter((x) => x.target !== undefined)
                    .map((x) => [x.path, x.target]),
                ),
              }),
              images.length,
            );
          const packages = rows.filter((x) => selected.has(x.id) && !x.path);
          if (queue.current && packages.length) {
            const task = await runTask("package-commit", {
              queueId: queue.current,
              items: packages.map((x) => ({
                id: x.id,
                applyMode: apply ? "applyAll" : "importOnly",
              })),
              useApplicationState: apply,
            });
            addResult(total, task, packages.length);
            // 成功提交会由后台释放队列；失败队列留给取消入口清理。
            if (task.status === "completed") queue.current = "";
          }
          if (queue.current) {
            await api.command(IPC.package_import_queue_cancel, {
              args: { queueId: queue.current },
            });
            queue.current = "";
          }
        }
      } catch (e) {
        total.errors.push(String(e));
        total.failed += Math.max(0, selected.size - total.success);
      } finally {
        if (!abort.current.signal.aborted) {
          setSummary(total);
          setProcessing(false);
        }
        waiting.current = false;
      }
    }, "处理结果已刷新");
  }
  const close = () => {
    if (!busy && !processing) onClose();
  };
  return (
    <Modal
      open
      onClose={close}
      title={
        mode === "export"
          ? "导出 Mod 包"
          : importKind === "images"
            ? "导入 PNG"
            : importKind === "packages"
              ? "导入加密包"
              : "导入资源"
      }
      wide
    >
      <div className="steps">
        {[
          mode === "import" ? "添加文件" : "选择格式",
          "识别与选择",
          "处理结果",
        ].map((s, i) => (
          <span className={step === i ? "active" : ""} key={s}>
            <b>{i < step ? <Check size={13} /> : i + 1}</b>
            {s}
          </span>
        ))}
      </div>
      {step === 0 ? (
        <div className="wizard-body">
          {mode === "import" ? (
            <>
              <button
                className="file-picker"
                onClick={() =>
                  void act(async () => {
                    const files = await api.chooseFiles(
                      target ? "images" : importKind,
                    );
                    if (files)
                      setPaths((p) => [
                        ...new Set([
                          ...p,
                          ...(Array.isArray(files) ? files : [files]),
                        ]),
                      ]);
                  })
                }
              >
                <FolderOpen size={38} />
                <strong>
                  {importKind === "images"
                    ? "选择 PNG 图片"
                    : importKind === "packages"
                      ? "选择加密 Mod 包"
                      : "选择图片或 Mod 包"}
                </strong>
                <span>支持多个文件，同部位多个候选与分卷包</span>
              </button>
              {paths.map((p) => (
                <div className="file-line" key={p}>
                  <FileImage size={16} />
                  <span>{p}</span>
                  <button
                    className="text-button"
                    onClick={() => setPaths(paths.filter((x) => x !== p))}
                  >
                    移除
                  </button>
                </div>
              ))}
            </>
          ) : (
            <>
              <label className="field">
                导出名称
                <input
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                />
              </label>
              <label className="field">
                格式
                <ThemedSelect
                  label="导出格式"
                  value={format}
                  onValueChange={setFormat}
                  options={[
                    { value: "apmod", label: "加密 Mod 包 · .apmod" },
                    { value: "zip", label: "普通图片包 · .zip" },
                  ]}
                />
              </label>
            </>
          )}
          {((mode === "export" && format === "apmod") ||
            uniquePackages(paths).length > 0) && (
            <label className="field">
              <span>
                <LockKeyhole size={14} />
                包密码
              </span>
              <input
                type="password"
                autoComplete="off"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="请输入包密码"
              />
            </label>
          )}
          {mode === "import" &&
            uniquePackages(paths).length > 1 &&
            uniquePackages(paths).map((path) => (
              <label className="field" key={path}>
                单包密码：{path.split(/[\\/]/).pop()}
                <input
                  type="password"
                  autoComplete="off"
                  value={passwords[path] || ""}
                  placeholder="留空使用默认包密码"
                  onChange={(e) =>
                    setPasswords((p) => ({ ...p, [path]: e.target.value }))
                  }
                />
              </label>
            ))}
          <div className="dialog-footer">
            <button
              className="button primary"
              disabled={busy || (mode === "import" && !paths.length)}
              onClick={() => void inspect()}
            >
              下一步 <ArrowRight size={16} />
            </button>
          </div>
        </div>
      ) : step === 1 ? (
        <div className="wizard-body">
          <div className="section-heading">
            <strong>
              {rows.length} 个文件 · 已选 {selected.size} 个
            </strong>
            <button
              className="text-button"
              onClick={() =>
                setSelected(
                  selected.size
                    ? new Set()
                    : new Set(
                        rows
                          .filter((x) => x.status !== "error")
                          .map((x) => x.id),
                      ),
                )
              }
            >
              全选 / 清空
            </button>
          </div>
          <PackageSelection
            rows={rows}
            selected={selected}
            onSelect={setSelected}
            resources={workspace.data?.resources || []}
            onTarget={(id, row) =>
              setRows((old) =>
                old.map((x) => (x.id === id ? classifyImageRow(x, row) : x)),
              )
            }
            queue={queue.current}
          />
          <label className="check-field">
            <input
              type="checkbox"
              checked={apply}
              onChange={(e) => setApply(e.target.checked)}
            />
            {mode === "export"
              ? "附带应用状态"
              : "应用包内所选 Mod（图片仅导入）"}
          </label>
          <div className="dialog-footer">
            <button
              className="button"
              disabled={busy}
              onClick={() => setStep(0)}
            >
              上一步
            </button>
            <button
              className="button primary"
              disabled={busy || !selected.size}
              onClick={() => void commit()}
            >
              {mode === "export" ? "开始导出" : "确认导入"}
            </button>
          </div>
        </div>
      ) : (
        <div className="wizard-body package-result" role="status">
          <h2>
            {processing
              ? "正在处理，请等待任务结束…"
              : summary.failed
                ? "处理结束，部分内容未完成"
                : "处理完成"}
          </h2>
          {processing ? (
            <>
              <progress />
              <p>任务 {taskId}</p>
              <button
                className="button"
                onClick={() =>
                  void act(() => api.cancel(taskId), "正在取消任务")
                }
              >
                取消当前任务
              </button>
            </>
          ) : (
            <>
              <p>
                成功 {summary.success} · 跳过 {summary.skipped} · 失败{" "}
                {summary.failed}
              </p>
              {summary.paths.map((p) => (
                <code key={p}>{p}</code>
              ))}
              {summary.errors.map((e, i) => (
                <p className="danger" key={i}>
                  {e}
                </p>
              ))}
              <button className="button primary" onClick={close}>
                返回工作区
              </button>
            </>
          )}
        </div>
      )}
      {notice && <div className="inline-notice">{notice}</div>}
    </Modal>
  );
}
