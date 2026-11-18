// 详情正文独立滚动，获取入口不随图片或长说明滚出视口。
import { useState } from "react";
import type { ListedWork } from "../../core/ipc/generated/ListedWork";
import { appService } from "../../core/services/app";
import { Badge, Modal } from "../../shared/Ui";
import { workStatus, workStatusLabel } from "../../core/domain/workshop";
import { WorkshopActions } from "./WorkshopActions";
import { WorkshopImage } from "./WorkshopImage";
import { WorkDate, WorkTags } from "./WorkCard";

export function WorkshopDetail({
  item,
  busy,
  notice,
  onClose,
  onAcquire,
  onRemove,
}: {
  item: ListedWork;
  busy: boolean;
  notice: string;
  onClose: () => void;
  onAcquire: () => void;
  onRemove: () => void;
}) {
  const [removing, setRemoving] = useState(false);
  const [linkError, setLinkError] = useState("");
  const work = item.work;
  const links = (urls: string[] | undefined, label: string) =>
    urls?.map((url, index) => (
      <button
        key={`${url}-${index}`}
        className="text-button work-credit-link"
        title={url}
        onClick={() => {
          setLinkError("");
          void appService
            .openExternal(url)
            .catch((error) => setLinkError(String(error)));
        }}
      >
        {label}
        {urls.length > 1 ? ` ${index + 1}` : ""} ↗
      </button>
    ));
  return (
    <>
      <Modal
        open
        wide
        title={work.title}
        className="workshop-detail"
        onClose={onClose}
      >
        <div className="work-detail-toolbar">
          <div>
            <Badge>{workStatusLabel[workStatus(item)]}</Badge>
            <span>v{work.version}</span>
            <WorkDate value={work.updatedAt} />
          </div>
          <div className="action-row">
            {item.installedVersion && (
              <button
                className="button"
                disabled={busy}
                onClick={() => setRemoving(true)}
              >
                移除作品
              </button>
            )}
            <WorkshopActions item={item} busy={busy} onAcquire={onAcquire} />
          </div>
        </div>
        {notice && (
          <p className="work-detail-notice" role="status">
            {notice}
          </p>
        )}
        <div className="work-detail-scroll">
          <div className="work-detail-intro">
            <aside>
              <WorkshopImage
                zoom
                id={work.id}
                name={work.cover}
                title={work.title}
                revision={work.revision}
              />
              <p className="preserve-lines">
                作者：{work.author || "作者未填写"}
              </p>
              <div className="work-credit-links">
                {links(work.authorUrls, "作者主页")}
              </div>
              {work.artist && (
                <p className="preserve-lines">画师：{work.artist}</p>
              )}
              <div className="work-credit-links">
                {links(work.artistUrls, "画师主页")}
              </div>
              <div className="work-credit-links">
                {links(work.sourceUrl ? [work.sourceUrl] : [], "作品来源")}
              </div>
              {linkError && <p role="alert">{linkError}</p>}
            </aside>
            <div className="work-detail-description">
              <WorkTags work={work} />
              {work.gameVersion && (
                <p className="muted">适用游戏版本：{work.gameVersion}</p>
              )}
              <p className="preserve-lines">{work.description}</p>
              {work.changeNote && (
                <>
                  <h3>本次更新</h3>
                  <p className="preserve-lines">{work.changeNote}</p>
                </>
              )}
            </div>
          </div>
          {!!work.previews.length && (
            <section>
              <h3>附图</h3>
              <div className="work-detail-previews">
                {[...new Set(work.previews)]
                  .filter((name) => name && name !== work.cover)
                  .map((name, index) => (
                    <WorkshopImage
                      zoom
                      key={name}
                      id={work.id}
                      name={name}
                      title={`${work.title} 附图 ${index + 1}`}
                      revision={work.revision}
                    />
                  ))}
              </div>
            </section>
          )}
          <section>
            <h3>资源范围</h3>
            {work.files.map((file) => (
              <div className="credit-row" key={file.name}>
                <span>{file.sourceName || file.name}</span>
                <small>{file.resource?.resourceName || file.canonical}</small>
              </div>
            ))}
          </section>
          <section>
            <h3>版本记录</h3>
            {work.history.length ? (
              work.history.map((h, index) => (
                <div key={index}>
                  <strong>{h.version}</strong>{" "}
                  <WorkDate value={h.publishedAt} />
                  <p className="preserve-lines">{h.changeNote}</p>
                </div>
              ))
            ) : (
              <p className="muted">暂无历史记录</p>
            )}
          </section>
        </div>
      </Modal>
      {removing && (
        <Modal
          open
          title={`确认移除「${work.title}」？`}
          onClose={() => !busy && setRemoving(false)}
        >
          <div className="wizard-body" role="alert">
            <p>
              会移除此作品拥有且未被修改的本地库资源，以及仍与作品一致的已应用游戏文件。其他来源和已修改文件会保留。
            </p>
            <div className="action-row">
              <button
                className="button"
                disabled={busy}
                onClick={() => setRemoving(false)}
              >
                取消移除
              </button>
              <button
                className="button danger"
                disabled={busy}
                onClick={onRemove}
              >
                确认移除作品
              </button>
            </div>
            {notice && <p role="status">{notice}</p>}
          </div>
        </Modal>
      )}
    </>
  );
}
