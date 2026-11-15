// 标题栏任务小窗只展示后端快照；关闭小窗不会取消任务或复制任务状态。
import * as Popover from "@radix-ui/react-popover";
import { IPC } from "../core/ipc/commands";
import type { TaskSnapshot } from "../core/ipc/generated/TaskSnapshot";
import { useActions } from "../hooks/useApp";
import { Badge } from "../shared/Ui";
import {
  CheckCircle2,
  LoaderCircle,
  XCircle,
  PauseCircle,
  ListTodo,
  X,
} from "lucide-react";
import { bytes } from "../core/domain/resources";
import { activeTask } from "../core/domain/tasks";
export function TaskPopover({
  open,
  onOpenChange,
  tasks: snapshots,
  loading,
  error,
  onRefresh,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tasks: TaskSnapshot[];
  loading: boolean;
  error: Error | null;
  onRefresh: () => void;
}) {
  const { api, act, busy, notice } = useActions();
  const tasks = [...snapshots].reverse();
  const running = tasks.filter(activeTask).length;
  return (
    <Popover.Root open={open} onOpenChange={onOpenChange}>
      <Popover.Trigger asChild>
        <button
          className="icon-button task-trigger"
          aria-label="任务进程"
          title="任务进程"
        >
          <ListTodo size={16} />
          {running > 0 && <span className="task-count">{running}</span>}
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          className="task-popover"
          side="bottom"
          align="end"
          sideOffset={8}
          collisionPadding={12}
          aria-labelledby="task-popover-title"
        >
          <div className="task-popover-heading">
            <h2 id="task-popover-title">任务进程</h2>
            <span>{running ? `${running} 项进行中` : "后台任务"}</span>
            <Popover.Close asChild>
              <button className="icon-button" aria-label="关闭任务进程">
                <X size={16} />
              </button>
            </Popover.Close>
          </div>
          <div className="task-popover-scroll">
            {error && (
              <div className="task-load-error" role="alert">
                读取任务失败：{error.message}
                <button className="button" onClick={onRefresh}>
                  重新读取
                </button>
              </div>
            )}
            {loading ? (
              <div className="task-empty" role="status">
                <LoaderCircle className="spin" size={24} />
                <p>正在读取任务…</p>
              </div>
            ) : !tasks.length && !error ? (
              <div className="task-empty">
                <ListTodo size={28} />
                <strong>暂无任务</strong>
                <p>下载、提取与安装的进度会显示在这里。</p>
              </div>
            ) : (
              <div className="task-list">
                {tasks.map((t) => (
                  <article className="task-card" key={t.id}>
                    <div className={`task-icon ${t.status}`}>
                      {activeTask(t) ? (
                        <LoaderCircle className="spin" />
                      ) : t.status === "completed" ? (
                        <CheckCircle2 />
                      ) : t.status === "paused" ? (
                        <PauseCircle />
                      ) : (
                        <XCircle />
                      )}
                    </div>
                    <div className="task-body">
                      <div className="section-heading">
                        <h3>{t.title}</h3>
                        <Badge tone={t.status === "completed" ? "success" : ""}>
                          {
                            (
                              {
                                running: "进行中",
                                completed: "已完成",
                                failed: "失败",
                                cancelled: "已取消",
                                queued: "排队中",
                                paused: "已暂停",
                              } as Record<string, string>
                            )[t.status]
                          }
                        </Badge>
                      </div>
                      <p>{t.message}</p>
                      {t.status === "running" && (
                        <progress
                          max={t.total || 100}
                          value={t.total ? t.done : undefined}
                        />
                      )}
                      <small>
                        {t.total
                          ? `${t.done.toFixed(0)} / ${t.total.toFixed(0)}`
                          : t.phase}
                      </small>
                      {!!t.speed && t.status === "running" && (
                        <small> · {bytes(t.speed)}/s</small>
                      )}
                      {!!t.result && t.status !== "running" && (
                        <details>
                          <summary>查看处理结果</summary>
                          <pre>{JSON.stringify(t.result, null, 2)}</pre>
                        </details>
                      )}
                    </div>
                    <div className="task-controls">
                      {t.canCancel && (
                        <button
                          className="button"
                          disabled={busy}
                          onClick={() =>
                            void act(
                              () => api.cancel(t.id),
                              "已请求取消，正在等待安全检查点。",
                            )
                          }
                        >
                          取消
                        </button>
                      )}
                      {t.canCancel &&
                        t.status === "running" &&
                        [
                          "preview",
                          "apx-cache",
                          "update-download",
                          "aux-install",
                        ].includes(t.kind) && (
                          <button
                            className="button"
                            disabled={busy}
                            onClick={() =>
                              void act(
                                () => api.command(IPC.task_pause, { id: t.id }),
                                "正在暂停，已下载内容将保留",
                              )
                            }
                          >
                            暂停
                          </button>
                        )}
                      {t.canRetry && (
                        <button
                          className="button primary"
                          disabled={busy}
                          onClick={() =>
                            void act(
                              () => api.command(IPC.task_retry, { id: t.id }),
                              "重试任务已加入队列",
                            )
                          }
                        >
                          {t.status === "paused" ? "继续" : "重试"}
                        </button>
                      )}
                    </div>
                  </article>
                ))}
              </div>
            )}
          </div>
          {notice && (
            <div className="inline-notice" role="status">
              {notice}
            </div>
          )}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
