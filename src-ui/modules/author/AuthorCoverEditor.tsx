// 主封面替换与附图管理分开呈现；实际图片写入由作者服务校验并提交。
import type { AuthorDraft } from "../../core/ipc/generated/AuthorDraft";
import { AuthorImage } from "./AuthorImage";

export function AuthorCoverEditor({
  draft,
  inherited,
  onCover,
  onAdd,
  onChange,
}: {
  draft: AuthorDraft;
  inherited: boolean;
  onCover: () => void;
  onAdd: () => void;
  onChange: (update: (draft: AuthorDraft) => AuthorDraft) => void;
}) {
  const cover = draft.images[0];
  return (
    <section className="author-cover-editor">
      <h3>主封面</h3>
      <div className="author-cover-main">
        {cover ? (
          <AuthorImage hash={cover.sha256} label="主封面" />
        ) : (
          <div className="author-cover-empty">
            {inherited ? "沿用已发布封面" : "暂无封面"}
          </div>
        )}
        <div>
          <button type="button" className="button" onClick={onCover}>
            {cover || inherited ? "替换封面" : "选择封面"}
          </button>
          <p className="muted">用于工坊卡片展示，建议使用方形 PNG。</p>
        </div>
      </div>
      <div className="author-toolbar">
        <h3>附图</h3>
        <button
          type="button"
          className="button"
          disabled={draft.images.length >= 7 || !cover}
          onClick={onAdd}
        >
          添加附图
        </button>
        <small>
          {inherited && !cover
            ? "未添加新图片时沿用已发布图片；替换时请补全需要保留的附图。"
            : "先选择封面，最多添加六张附图。资源原图保持原样。"}
        </small>
      </div>
      {draft.images.slice(1).map((file, index) => (
        <div className="author-file" key={file.id}>
          <AuthorImage hash={file.sha256} label={`附图 ${index + 1}`} />
          <span>附图 {index + 1}</span>
          <button
            className="text-button"
            type="button"
            onClick={() =>
              onChange((d) => ({
                ...d,
                images: [file, ...d.images.filter((f) => f.id !== file.id)],
              }))
            }
          >
            设为封面
          </button>
          <button
            className="text-button"
            type="button"
            onClick={() =>
              onChange((d) => ({
                ...d,
                images: d.images.filter((f) => f.id !== file.id),
              }))
            }
          >
            移除附图
          </button>
        </div>
      ))}
    </section>
  );
}
