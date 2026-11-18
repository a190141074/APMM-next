// 草稿和不可变投稿快照共用工坊展示组件，预览不会触发获取或修改内容。
import { useState } from "react";
import type { AuthorDraft } from "../../core/ipc/generated/AuthorDraft";
import type { Work } from "../../core/ipc/generated/Work";
import {
  WorkCard,
  LayoutSwitch,
  type WorkshopLayout,
} from "../workshop/WorkCard";
import { WorkshopImage } from "../workshop/WorkshopImage";
import { AuthorImage } from "./AuthorImage";

export function AuthorCardPreview({
  draft,
  published,
}: {
  draft: AuthorDraft;
  published?: Work | null;
}) {
  const [layout, setLayout] = useState<WorkshopLayout>("horizontal");
  const cover = draft.images[0];
  const meta = draft.metadata;
  return (
    <section className="author-card-preview">
      <h3>工坊展示预览</h3>
      <LayoutSwitch value={layout} onChange={setLayout} />
      <div className={`work-layout-${layout}`}>
        <WorkCard
          work={{
            ...meta,
            updatedAt: 0,
            tags: meta.tags.map((name) => ({ name, highlight: false })),
          }}
          image={
            cover ? (
              <AuthorImage hash={cover.sha256} label="封面预览" />
            ) : published?.cover ? (
              <WorkshopImage
                id={published.id}
                name={published.cover}
                title={meta.title}
                revision={published.revision}
              />
            ) : (
              <div className="workshop-image">
                <span>暂无封面</span>
              </div>
            )
          }
        />
      </div>
    </section>
  );
}
