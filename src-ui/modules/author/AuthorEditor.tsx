// 草稿编辑、来源选择和提交预览各有独立边界，分类选择引用现有资源槽位。
import { useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { authorService } from "../../core/services/author";
import type { Work } from "../../core/ipc/generated/Work";
import type { AuthorDraft } from "../../core/ipc/generated/AuthorDraft";
import type { AuthorPreview } from "../../core/ipc/generated/AuthorPreview";
import { useAuthorDraft } from "../../hooks/useAuthorDraft";
import { useActions, useWorkspace } from "../../hooks/useApp";
import { Modal } from "../../shared/Ui";
import { ThemedSelect } from "../../shared/ThemedSelect";
import { bytes } from "../../core/domain/resources";
import { AuthorMetadataFields } from "./AuthorMetadataFields";
import { AuthorCoverEditor } from "./AuthorCoverEditor";
import { AuthorCardPreview } from "./AuthorCardPreview";
export function AuthorEditor({
  initial,
  onBack,
}: {
  initial: AuthorDraft;
  onBack: () => void;
}) {
  const session = useAuthorDraft(initial);
  const { draft } = session;
  const { api, act, busy, notice } = useActions();
  const workspace = useWorkspace();
  const author = useQuery({
    queryKey: ["author-state"],
    queryFn: authorService.state,
  });
  const published = author.data?.works.find(
    (work) => work.id === draft.workId,
  )?.published;
  const [preview, setPreview] = useState<AuthorPreview | null>(null);
  const [previewPublished, setPreviewPublished] = useState<Work | null>(null);
  const [library, setLibrary] = useState(false);
  const [chosen, setChosen] = useState<string[]>([]);
  const [path, setPath] = useState("");
  const [imageSource, setImageSource] = useState(false);
  const password = useRef<HTMLInputElement>(null);
  const addSource = (previewImage: boolean) =>
    void act(async () => {
      await session.flush();
      const files = await api.chooseFiles(previewImage ? "images" : "author");
      const selected = Array.isArray(files) ? files[0] : files;
      if (selected) {
        setImageSource(previewImage);
        setPath(selected);
      }
    }, "");
  const setCover = () =>
    void act(async () => {
      await session.flush();
      const selected = await api.chooseFiles("images");
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (!path) return;
      await session.flush();
      const current = session.current();
      session.accept(
        await authorService.cover(current.id, current.revision, path),
      );
    }, "");
  return (
    <>
      <div className="author-toolbar">
        <button
          className="button"
          onClick={() =>
            void act(async () => {
              await session.flush();
              onBack();
            }, "")
          }
        >
          返回草稿
        </button>
        <small role="status">
          {session.error ||
            (session.saving
              ? "正在保存…"
              : session.dirty
                ? "等待保存"
                : "草稿已保存")}
        </small>
        {session.error && (
          <button className="button" onClick={() => void act(session.flush)}>
            重试保存
          </button>
        )}
      </div>
      <fieldset disabled={busy} className="author-editor-fields">
        <AuthorMetadataFields
          value={draft.metadata}
          onChange={(metadata) => session.edit((d) => ({ ...d, metadata }))}
        />
        <label className="check">
          <input
            type="checkbox"
            checked={draft.metadataOnly}
            disabled={!published}
            onChange={(e) =>
              session.edit((d) => ({ ...d, metadataOnly: e.target.checked }))
            }
          />
          仅更新资料（已发布作品）
        </label>
        {published && !draft.metadataOnly && (
          <p className="inline-notice">
            内容更新会以本次完整资源清单替换旧版。请添加所有需要保留的资源，并在预检中核对删除项。
          </p>
        )}
        <h3>作品资源</h3>
        <div className="author-toolbar">
          <button
            className="button"
            disabled={draft.metadataOnly}
            onClick={() => addSource(false)}
          >
            添加 PNG / ZIP / .apmod
          </button>
          <button
            className="button"
            disabled={draft.metadataOnly}
            onClick={() => {
              setChosen([]);
              setLibrary(true);
            }}
          >
            从本地库选择
          </button>
        </div>
        <div className="author-files">
          {draft.files.map((file) => (
            <div className="author-file" key={file.id}>
              <div>
                <strong>{file.sourceName}</strong>
                <small>
                  {bytes(file.bytes)} · {file.sha256.slice(0, 12)}
                </small>
              </div>
              <ThemedSelect
                label={`${file.sourceName} 应用槽位`}
                value={file.canonical}
                onValueChange={(value) =>
                  session.edit((d) => ({
                    ...d,
                    files: d.files.map((f) =>
                      f.id === file.id ? { ...f, canonical: value } : f,
                    ),
                  }))
                }
                options={[
                  { value: "", label: "请选择分类" },
                  ...(() => {
                    const entries = new Map(
                      (workspace.data?.resources || [])
                        .filter((r) => r.applicationSlot)
                        .map((r) => [
                          r.canonical,
                          {
                            value: r.canonical,
                            label: `${r.entityName} · ${r.resourceLabel || r.canonical}`,
                          },
                        ]),
                    );
                    if (file.canonical && !entries.has(file.canonical))
                      entries.set(file.canonical, {
                        value: file.canonical,
                        label: file.canonical,
                      });
                    return [...entries.values()];
                  })(),
                ]}
              />
              <button
                className="text-button"
                onClick={() =>
                  session.edit((d) => ({
                    ...d,
                    files: d.files.filter((f) => f.id !== file.id),
                  }))
                }
              >
                移除
              </button>
            </div>
          ))}
        </div>
        <AuthorCoverEditor
          draft={draft}
          inherited={!!published}
          onCover={setCover}
          onAdd={() => addSource(true)}
          onChange={session.edit}
        />
        <AuthorCardPreview draft={draft} published={published} />
      </fieldset>
      <p role="status">{notice}</p>
      <p className="muted">
        导出后，通过约定的收稿渠道发送 ZIP。草稿和导出无需服务器。
      </p>
      <div className="author-toolbar">
        <button
          className="button primary"
          disabled={busy}
          onClick={() =>
            void act(async () => {
              await session.flush();
              const next = await authorService.preflight(draft.id);
              session.accept(next.draft);
              setPreviewPublished(
                published ? structuredClone(published) : null,
              );
              setPreview(next);
            }, "投稿预检完成")
          }
        >
          预览投稿与导出
        </button>
      </div>
      {path && (
        <Modal open title="解析本地来源" onClose={() => setPath("")}>
          <div className="wizard-body">
            <p>{path.split(/[\\/]/).pop()}</p>
            {!imageSource && (
              <label className="field">
                包密码（加密包填写）
                <input ref={password} type="password" autoComplete="off" />
              </label>
            )}
            <p role="status">{notice}</p>
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void act(async () => {
                  const secret = password.current?.value || "";
                  if (password.current) password.current.value = "";
                  await session.flush();
                  session.accept(
                    await authorService.source(
                      draft.id,
                      path,
                      secret,
                      imageSource,
                    ),
                  );
                  setPath("");
                }, "来源已添加")
              }
            >
              解析并添加
            </button>
          </div>
        </Modal>
      )}
      {library && (
        <Modal
          open
          wide
          title="从本地库选择资源"
          onClose={() => setLibrary(false)}
        >
          <div className="wizard-body">
            {(workspace.data?.resources || []).flatMap((r) =>
              r.files.map((f) => (
                <label className="author-file" key={f.path}>
                  <input
                    type="checkbox"
                    checked={chosen.includes(f.path)}
                    onChange={(e) =>
                      setChosen((old) =>
                        e.target.checked
                          ? [...old, f.path]
                          : old.filter((p) => p !== f.path),
                      )
                    }
                  />
                  <div>
                    {f.sourceName}
                    <small>
                      {r.entityName} · {r.resourceLabel} · {f.hash.slice(0, 12)}
                    </small>
                  </div>
                </label>
              )),
            )}
            <p>{notice}</p>
          </div>
          <footer className="wizard-footer">
            <button
              className="button primary"
              disabled={busy || !chosen.length}
              onClick={() =>
                void act(async () => {
                  await session.flush();
                  session.accept(await authorService.library(draft.id, chosen));
                  setLibrary(false);
                }, "库资源已添加")
              }
            >
              添加所选资源
            </button>
          </footer>
        </Modal>
      )}
      {preview && (
        <Modal open wide title="确认投稿快照" onClose={() => setPreview(null)}>
          <div className="wizard-body">
            <AuthorCardPreview
              draft={preview.draft}
              published={previewPublished}
            />
            <h3>
              {preview.draft.metadata.title} · v{preview.draft.metadata.version}
            </h3>
            <p>
              {preview.draft.metadataOnly ? "仅更新资料" : "资料及内容更新"} ·{" "}
              {bytes(preview.bytes)}
            </p>
            <p>
              新增 {preview.added.length} · 修改 {preview.changed.length} · 删除{" "}
              {preview.removed.length}
            </p>
            {[
              ["新增", preview.added],
              ["修改", preview.changed],
              ["删除", preview.removed],
            ].map(([label, names]) => (
              <p key={String(label)}>
                {String(label)}：{(names as string[]).join("、") || "无"}
              </p>
            ))}
            {preview.draft.files.map((file) => (
              <div className="credit-row" key={file.id}>
                <span>{file.sourceName}</span>
                <small>
                  {file.canonical} · {bytes(file.bytes)}
                </small>
              </div>
            ))}
            <p>
              资料{preview.metadataChanged ? "有修改" : "未修改"}。投稿编号：
              {preview.id}
            </p>
            <p>
              此包包含作品资料与原始资源，不包含包密码或管理凭证。导出不代表已交稿。
            </p>
            <p role="status">{notice}</p>
          </div>
          <footer className="wizard-footer">
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void act(async () => {
                  const name =
                    preview.draft.metadata.title
                      .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_")
                      .slice(0, 100) || "投稿";
                  const dest = await api.saveFile(
                    `${name}-${preview.id.slice(0, 8)}.zip`,
                  );
                  if (dest) {
                    await authorService.export(preview.id, dest);
                    await author.refetch();
                  } else throw new Error("已取消导出，草稿已保留");
                }, "投稿包已导出，请通过约定渠道交给维护者")
              }
            >
              导出投稿包
            </button>
          </footer>
        </Modal>
      )}
    </>
  );
}
