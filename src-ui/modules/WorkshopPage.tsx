// 工坊浏览与获取确认独立于辅助插件管理，后台任务不随页面卸载。
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { workshopService } from "../core/services/workshop";
import {
  filterWorks,
  workStatus,
  workStatusLabel,
} from "../core/domain/workshop";
import { useActions } from "../hooks/useApp";
import { Badge, Empty, Loading } from "../shared/Ui";
import { ThemedSelect } from "../shared/ThemedSelect";
import { WorkshopImage } from "./workshop/WorkshopImage";
import { InstallDialog } from "./workshop/InstallDialog";
import type { ListedWork } from "../core/ipc/generated/ListedWork";
import type { InstallPlan } from "../core/ipc/generated/InstallPlan";
import {
  WorkCard,
  LayoutSwitch,
  type WorkshopLayout,
} from "./workshop/WorkCard";
import { WorkshopActions } from "./workshop/WorkshopActions";
import { WorkshopDetail } from "./workshop/WorkshopDetail";
import { IPC } from "../core/ipc/commands";
export function WorkshopPage({ onImport }: { onImport: () => void }) {
  const q = useQuery({
    queryKey: [IPC.workshop_list],
    queryFn: workshopService.list,
  });
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("");
  const [status, setStatus] = useState("");
  const [sort, setSort] = useState("updated");
  const [detailId, setDetailId] = useState<string | null>(null);
  const [layout, setLayout] = useState<WorkshopLayout>(() => {
    try {
      return localStorage.getItem("apmm.workshop-layout") === "grid"
        ? "grid"
        : "horizontal";
    } catch {
      return "horizontal";
    }
  });
  const changeLayout = (next: WorkshopLayout) => {
    setLayout(next);
    try {
      localStorage.setItem("apmm.workshop-layout", next);
    } catch {
      /* 存储不可用时仍可切换当前视图。 */
    }
  };
  const [plan, setPlan] = useState<InstallPlan | null>(null);
  const { act, busy, notice } = useActions();
  const items = q.data?.works || [];
  const detail = items.find((item) => item.work.id === detailId);
  const acquire = (item: ListedWork) =>
    void act(async () => {
      setPlan(await workshopService.preflight(item.work.id));
    }, "");
  const visible = filterWorks(items, search, category, status, sort);
  const unavailable = q.isError || (q.data && !q.data.available);
  return (
    <div className="workshop-page">
      <div className="page-heading">
        <div>
          <h1>创意工坊</h1>
          <p>发现社区作品，加入本地库。</p>
        </div>
        <button className="button" onClick={() => void q.refetch()}>
          刷新
        </button>
      </div>
      {q.isPending ? (
        <Loading />
      ) : unavailable ? (
        <Empty
          title={
            q.data?.status === "unconfigured"
              ? "工坊尚未配置"
              : "暂时无法连接工坊"
          }
          detail={q.data?.message || "连接失败，请稍后重试。"}
        >
          <div className="action-row">
            <button className="button" onClick={() => void q.refetch()}>
              重试
            </button>
            <button className="button" onClick={onImport}>
              导入本地包
            </button>
          </div>
        </Empty>
      ) : !items.length ? (
        <Empty title="工坊暂时没有作品" />
      ) : (
        <>
          <div className="filterbar workshop-filters">
            <input
              aria-label="搜索工坊"
              placeholder="作品、作者、标签…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
            <ThemedSelect
              label="作品分类"
              value={category}
              onValueChange={setCategory}
              options={[
                { value: "", label: "全部分类" },
                ...[...new Set(items.map((i) => i.work.category))]
                  .sort()
                  .map((c) => ({ value: c, label: c || "未分类" })),
              ]}
            />
            <ThemedSelect
              label="获取状态"
              value={status}
              onValueChange={setStatus}
              options={[
                { value: "", label: "全部状态" },
                ...Object.entries(workStatusLabel).map(([value, label]) => ({
                  value,
                  label,
                })),
              ]}
            />
            <ThemedSelect
              label="作品排序"
              value={sort}
              onValueChange={setSort}
              options={[
                { value: "updated", label: "最近更新" },
                { value: "name", label: "名称排序" },
              ]}
            />
            <LayoutSwitch value={layout} onChange={changeLayout} />
          </div>
          {!visible.length ? (
            <Empty title="没有符合条件的作品" />
          ) : (
            <div className={`workshop-grid work-layout-${layout}`}>
              {visible.map((item) => (
                <WorkCard
                  key={item.work.id}
                  work={item.work}
                  onOpen={() => setDetailId(item.work.id)}
                  image={
                    <WorkshopImage
                      id={item.work.id}
                      name={item.work.cover}
                      title={item.work.title}
                      revision={item.work.revision}
                    />
                  }
                  status={<Badge>{workStatusLabel[workStatus(item)]}</Badge>}
                  actions={
                    <WorkshopActions
                      item={item}
                      busy={busy}
                      onAcquire={() => acquire(item)}
                    />
                  }
                />
              ))}
            </div>
          )}
        </>
      )}
      {notice && (
        <p role="status" className="inline-notice">
          {notice}
        </p>
      )}
      {detail && !plan && (
        <WorkshopDetail
          key={detail.work.id}
          item={detail}
          busy={busy}
          notice={notice}
          onClose={() => setDetailId(null)}
          onAcquire={() => acquire(detail)}
          onRemove={() =>
            void act(async () => {
              await workshopService.uninstall(detail.work.id);
              setDetailId(null);
            }, "移除已加入后台任务，请查看任务结果")
          }
        />
      )}
      {plan && (
        <InstallDialog
          plan={plan}
          onClose={() => {
            setPlan(null);
            setDetailId(null);
          }}
        />
      )}
    </div>
  );
}
