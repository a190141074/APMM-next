// 作者工作区以离线交稿为主；导出记录不代表维护者已接收或发布。
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { authorService } from "../core/services/author";
import { useActions } from "../hooks/useApp";
import { AuthorEditor } from "./author/AuthorEditor";
import { Empty, Loading, Badge } from "../shared/Ui";
import type { AuthorDraft } from "../core/ipc/generated/AuthorDraft";
const statuses: Record<string, string> = {
  exported: "已导出",
  pending: "待审核",
  approved: "已批准待发布",
  publishing: "发布中",
  published: "已发布",
  rejected: "已拒绝",
  cancelled: "已取消",
  withdrawn: "已撤回",
};
export function AuthorPage() {
  const q = useQuery({
    queryKey: ["author-state"],
    queryFn: authorService.state,
  });
  const [tab, setTab] = useState("drafts");
  const [draft, setDraft] = useState<AuthorDraft | null>(null);
  const { act, busy, notice } = useActions();
  return (
    <div className="author-page">
      <div className="page-heading">
        <div>
          <h1>投稿</h1>
          <p>本地编辑并导出投稿包，通过约定渠道交给维护者审核。</p>
        </div>
      </div>
      {draft ? (
        <AuthorEditor
          key={draft.id}
          initial={draft}
          onBack={() => {
            setDraft(null);
            void q.refetch();
          }}
        />
      ) : (
        <>
          <div className="author-tabs" role="tablist">
            {[
              ["works", "我的作品"],
              ["drafts", "草稿"],
              ["receipts", "导出与投稿记录"],
            ].map(([id, label]) => (
              <button
                role="tab"
                aria-selected={tab === id}
                key={id}
                onClick={() => setTab(id)}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="author-toolbar">
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void act(async () => setDraft(await authorService.create()), "")
              }
            >
              新建作品
            </button>
            <button className="button" onClick={() => void q.refetch()}>
              刷新本地记录
            </button>
          </div>
          {q.isPending ? (
            <Loading />
          ) : q.error ? (
            <Empty title="作者记录读取失败" detail={String(q.error)} />
          ) : tab === "drafts" ? (
            <div>
              {q.data?.drafts.length ? (
                q.data.drafts.map((d) => (
                  <div className="credit-row" key={d.id}>
                    <div>
                      <strong>{d.metadata.title || "未命名草稿"}</strong>
                      <small>{d.files.length} 个资源</small>
                    </div>
                    <button className="button" onClick={() => setDraft(d)}>
                      编辑
                    </button>
                  </div>
                ))
              ) : (
                <Empty
                  title="暂无草稿"
                  detail="新建作品后可添加本地资源，或导入以前导出的投稿 ZIP。"
                />
              )}
            </div>
          ) : tab === "works" ? (
            <div>
              {q.data?.works.length ? (
                q.data.works.map((w) => (
                  <div className="credit-row" key={w.id}>
                    <div>
                      <strong>{w.title}</strong>
                      <small>
                        {w.published
                          ? `已发布 ${w.published.version}`
                          : "尚未同步到公开版本"}{" "}
                        · {w.id}
                      </small>
                    </div>
                    <button
                      className="button"
                      disabled={busy}
                      onClick={() =>
                        void act(async () => {
                          await authorService.syncPublished(w.id);
                          await q.refetch();
                        }, "已从签名目录同步公开版本")
                      }
                    >
                      同步公开版本
                    </button>
                    <button
                      className="button"
                      disabled={busy || !w.published}
                      onClick={() =>
                        void act(
                          async () =>
                            setDraft(await authorService.create(w.id)),
                          "",
                        )
                      }
                    >
                      新建更新草稿
                    </button>
                  </div>
                ))
              ) : (
                <Empty
                  title="还没有作品记录"
                  detail="首次预检会分配作品编号，无需注册或连接投稿服务器。"
                />
              )}
            </div>
          ) : (
            <div>
              {q.data?.receipts.length ? (
                q.data.receipts.map((r) => (
                  <div className="credit-row" key={r.id}>
                    <div>
                      <strong>{r.id}</strong>
                      <small>SHA-256：{r.snapshotHash}</small>
                      <small>{r.message}</small>
                    </div>
                    <Badge>{statuses[r.status] || r.status}</Badge>
                  </div>
                ))
              ) : (
                <Empty title="暂无导出或投稿记录" />
              )}
              <p className="muted">
                离线包请自行交给维护者；收稿、审核结果和撤回申请通过原收稿渠道沟通。
              </p>
            </div>
          )}
          {notice && <p role="status">{notice}</p>}
        </>
      )}
    </div>
  );
}
