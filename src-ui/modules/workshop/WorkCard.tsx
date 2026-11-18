// 工坊与投稿预览共用的作品展示，图片来源和操作由各自流程传入。
import type { ReactNode } from "react";
import type { Work } from "../../core/ipc/generated/Work";

export type WorkPresentation = Pick<
  Work,
  | "title"
  | "author"
  | "description"
  | "category"
  | "tags"
  | "version"
  | "updatedAt"
>;
export type WorkshopLayout = "horizontal" | "grid";

export function WorkTags({ work }: { work: Pick<Work, "category" | "tags"> }) {
  return (
    <div className="work-tags">
      {work.category && <span className="work-category">{work.category}</span>}
      {work.tags.map((tag) => (
        <span
          key={tag.name}
          className={tag.highlight ? "work-tag highlighted" : "work-tag"}
        >
          {tag.name}
        </span>
      ))}
    </div>
  );
}

export function WorkDate({ value }: { value: number }) {
  if (!value) return null;
  const date = new Date(Number(value) * 1000);
  if (!Number.isFinite(date.getTime())) return null;
  return (
    <time dateTime={date.toISOString()} title={date.toLocaleString("zh-CN")}>
      更新于 {date.toLocaleDateString("zh-CN")}
    </time>
  );
}

export function LayoutSwitch({
  value,
  onChange,
}: {
  value: WorkshopLayout;
  onChange: (value: WorkshopLayout) => void;
}) {
  return (
    <div className="work-layout-switch" role="group" aria-label="工坊视图">
      <button
        type="button"
        aria-pressed={value === "horizontal"}
        onClick={() => onChange("horizontal")}
      >
        横向卡片
      </button>
      <button
        type="button"
        aria-pressed={value === "grid"}
        onClick={() => onChange("grid")}
      >
        封面网格
      </button>
    </div>
  );
}

export function WorkCard({
  work,
  image,
  status,
  actions,
  onOpen,
}: {
  work: WorkPresentation;
  image: ReactNode;
  status?: ReactNode;
  actions?: ReactNode;
  onOpen?: () => void;
}) {
  return (
    <article
      className="workshop-card"
      onClick={(e) => {
        if (!(e.target as HTMLElement).closest("button, a")) onOpen?.();
      }}
    >
      <div className="work-card-main">
        <div className="work-card-cover">{image}</div>
        <div className="work-card-info">
          <div className="work-card-heading">
            <h3 className="work-card-title" title={work.title}>
              {onOpen ? (
                <button type="button" onClick={onOpen}>
                  {work.title}
                </button>
              ) : (
                work.title || "作品名称"
              )}
            </h3>
            {status && <div className="work-card-status">{status}</div>}
          </div>
          <div className="work-card-author" title={work.author || "作者未填写"}>
            作者：{work.author || "作者未填写"}
          </div>
          <p className="work-card-description">
            {work.description || "暂无简介"}
          </p>
          <WorkTags work={work} />
        </div>
      </div>
      <footer className="work-card-footer">
        <div className="work-card-meta">
          <span>v{work.version || "未填写"}</span>
          <WorkDate value={work.updatedAt} />
        </div>
        {actions && <div className="work-card-actions">{actions}</div>}
      </footer>
    </article>
  );
}
