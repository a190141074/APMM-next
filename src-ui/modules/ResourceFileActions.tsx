// 文件管理与应用点击分离，删除前显示明确的候选身份并等待确认。
import { useState } from "react";
import { Check, FolderOpen, Download, Power } from "lucide-react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import type { ModFile } from "../core/ipc/generated/ModFile";
import { IPC } from "../core/ipc/commands";
import { useActions, useWorkspace } from "../hooks/useApp";
import { slotOfResource, storageCategory } from "../core/domain/resources";
import { Modal } from "../shared/Ui";
import { ResourceSlotSelect } from "../shared/ResourceSlotSelect";
import { CandidateCreditsFields } from "../shared/CandidateCreditsFields";
import { CandidateComparison } from "../shared/CandidateComparison";
import {
  candidateName,
  candidateExportName,
  normalizedCandidateInfo,
} from "../core/domain/candidateInfo";
export function ResourceFileActions({
  row,
  file,
  onClose,
}: {
  row: ResourceRecord;
  file: ModFile;
  onClose: () => void;
}) {
  const { api, act, busy, notice } = useActions();
  const [savedName, setSavedName] = useState(file.sourceName);
  const [name, setName] = useState(candidateName(file.sourceName));
  const [deleting, setDeleting] = useState(false);
  const workspace = useWorkspace();
  const [info, setInfo] = useState(() => normalizedCandidateInfo(file.info));
  const [target, setTarget] = useState<ResourceRecord | null>(
    row.category === "temporary" ? null : row,
  );
  const [moving, setMoving] = useState(false);
  const pending = file.classification?.pending;
  const currentRow =
    workspace.data?.resources.find((r) => r.key === row.key) || row;
  const currentFile =
    currentRow.files.find(
      (f) => f.hash === file.hash || f.path === file.path,
    ) || file;
  const args = {
    heroId: row.entityId,
    sectionKey: row.section,
    canonical: row.canonical,
    hash: file.hash,
  };
  const saveName = () => {
    if (busy || !name.trim()) return;
    void act(async () => {
      const saved = await api.command<string>(IPC.rename_file, {
        args: { ...args, name },
      });
      setSavedName(saved);
      setName(candidateName(saved));
    }, "名称已保存");
  };
  return (
    <Modal
      open
      wide
      onClose={onClose}
      title="模组信息"
      className="mod-info-modal"
    >
      <div className="mod-info-layout">
        <CandidateComparison row={currentRow} file={currentFile} />
        <div className="wizard-body mod-info-body">
          <div className="mod-info-heading">
            <label className="field mod-name-editor">
              候选名称
              <span>
                <input
                  aria-label="候选名称"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.nativeEvent.isComposing) {
                      e.preventDefault();
                      saveName();
                    }
                    if (e.key === "Escape") setName(candidateName(savedName));
                  }}
                />
                <button
                  className="icon-button"
                  aria-label="保存名称"
                  title="保存名称（Enter）"
                  disabled={busy || !name.trim()}
                  onClick={saveName}
                >
                  <Check size={16} />
                </button>
              </span>
            </label>
            <div className="action-row">
              <button
                className="icon-button"
                aria-label="打开文件位置"
                title="打开文件位置"
                onClick={() =>
                  void act(() => api.command(IPC.reveal_mod_file, { args }))
                }
              >
                <FolderOpen size={16} />
              </button>
              <button
                className="icon-button"
                aria-label="导出图片"
                title="导出图片"
                onClick={() =>
                  void act(async () => {
                    const destination = await api.saveFile(
                      candidateExportName(savedName),
                    );
                    if (destination)
                      await api.task("quick-export", "导出图片", {
                        destination,
                        mode: "file",
                        items: [
                          {
                            category: storageCategory(row),
                            entityId: row.entityId,
                            sectionKey: row.section,
                            canonical: row.canonical,
                            hash: file.hash,
                          },
                        ],
                      });
                  })
                }
              >
                <Download size={16} />
              </button>
              <button
                className="icon-button"
                aria-label={currentRow.disabled ? "启用部位" : "禁用部位"}
                title={currentRow.disabled ? "启用部位" : "禁用部位"}
                onClick={() =>
                  void act(() =>
                    api.command(IPC.mod_set_disabled, {
                      args,
                      disabled: !currentRow.disabled,
                    }),
                  )
                }
              >
                <Power size={16} />
              </button>
            </div>
          </div>
          <details className="mod-file-details">
            <summary>文件详情</summary>
            <code className="resource-code">{file.hash}</code>
            <p className="resource-code">{file.path}</p>
          </details>
          <CandidateCreditsFields
            info={info}
            onChange={setInfo}
            disabled={busy}
            onOpen={(url) => void act(() => api.openExternal(url))}
          />
          <div className="action-row">
            <button
              className="button"
              disabled={busy}
              onClick={() =>
                void act(() =>
                  api.command(IPC.candidate_info_set, { args, info }),
                )
              }
            >
              保存信息
            </button>
          </div>
          <h3>归属</h3>
          <ResourceSlotSelect
            resources={workspace.data?.resources || []}
            value={target?.key || ""}
            onChange={(r) => {
              setTarget(r);
              setMoving(false);
            }}
            allowTemporary={row.category === "temporary"}
            disabled={busy}
          />
          {pending && (
            <div className="inline-notice">
              <p>
                新规则建议归属：{pending.canonical}。当前归属保留，等待确认。
              </p>
              <div className="action-row">
                <button
                  className="button"
                  disabled={busy}
                  onClick={() =>
                    void act(async () => {
                      await api.command(IPC.candidate_classify, {
                        args,
                        target: pending,
                        keepCurrent: true,
                      });
                      onClose();
                    })
                  }
                >
                  保留当前归属
                </button>
                <button
                  className="button"
                  disabled={busy}
                  onClick={() =>
                    void act(async () => {
                      await api.command(IPC.candidate_classify, {
                        args,
                        target: pending,
                        keepCurrent: false,
                      });
                      onClose();
                    })
                  }
                >
                  采用规则结果
                </button>
              </div>
            </div>
          )}
          {target &&
            target.key !== row.key &&
            (moving ? (
              <div className="inline-notice">
                <p>
                  将“{file.sourceName}”归类到 {target.entityName} ·{" "}
                  {target.variantLabel} · {target.resourceLabel}（
                  {target.canonical}）。仅移动库候选，预设保留原记录。
                </p>
                <button
                  className="button primary"
                  disabled={busy}
                  onClick={() =>
                    void act(async () => {
                      await api.command(IPC.candidate_classify, {
                        args,
                        target: slotOfResource(target),
                        keepCurrent: false,
                      });
                      onClose();
                    })
                  }
                >
                  确认归类
                </button>
                <button className="button" onClick={() => setMoving(false)}>
                  取消
                </button>
              </div>
            ) : (
              <button
                className="button"
                disabled={
                  busy || (row.category === "temporary" && row.disabled)
                }
                onClick={() => setMoving(true)}
              >
                更改归属
              </button>
            ))}
          {deleting ? (
            <div className="inline-notice">
              <p>
                确认删除“{file.sourceName}
                ”？如果正在应用，会同时恢复该部位原版。
              </p>
              <button
                className="button danger"
                disabled={busy}
                onClick={() =>
                  void act(async () => {
                    await api.command(IPC.remove_file, { args });
                    onClose();
                  })
                }
              >
                确认删除此候选
              </button>
              <button className="button" onClick={() => setDeleting(false)}>
                取消
              </button>
            </div>
          ) : (
            <button
              className="text-button danger"
              onClick={() => setDeleting(true)}
            >
              删除此候选
            </button>
          )}
          {notice && <div role="status">{notice}</div>}
        </div>
      </div>
    </Modal>
  );
}
