// 原生拖入反馈独立于页面筛选；悬停请求带代次，关闭和切页时丢弃旧结果。
import { useCallback, useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { appService } from "../core/services/app";
import { useDrop } from "./useApp";
import {
  dropSummary,
  planDrop,
  rowTarget,
  type DropEvent,
} from "../core/domain/dropImport";
import type { ImageDropMatch } from "../core/ipc/generated/ImageDropMatch";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import type { ImportTarget } from "../core/domain/packages";

export function useResourceDrop({
  resources,
  route,
  blocked,
  onUnknown,
  onPackages,
  notice,
}: {
  resources: ResourceRecord[];
  route: string;
  blocked: boolean;
  onUnknown: (paths: string[], packages: string[]) => void;
  onPackages: (paths: string[]) => void;
  notice: (message: string) => void;
}) {
  const client = useQueryClient();
  const [feedback, setFeedback] = useState<{
    message: string;
    keys: string[];
  } | null>(null);
  const current = useRef({ resources, blocked, onUnknown, onPackages, notice });
  current.current = { resources, blocked, onUnknown, onPackages, notice };
  const session = useRef({
    generation: 0,
    paths: [] as string[],
    matches: [] as ImageDropMatch[],
    position: undefined as DropEvent["position"],
    active: false,
    ready: false,
  });
  const busy = useRef(false);
  const [importing, setImporting] = useState(false);
  const abort = useRef(new AbortController());
  const clear = useCallback(() => {
    session.current.generation++;
    session.current.active = false;
    session.current.paths = [];
    session.current.matches = [];
    session.current.ready = false;
    setFeedback(null);
  }, []);
  const landing = useCallback(
    (position: DropEvent["position"]): ImportTarget | undefined => {
      if (!position) return;
      const key = document
        .elementFromPoint(position.x, position.y)
        ?.closest<HTMLElement>("[data-drop-resource-key]")
        ?.dataset.dropResourceKey;
      const row = current.current.resources.find((r) => r.key === key);
      return row ? rowTarget(row) : undefined;
    },
    [],
  );
  const show = useCallback(() => {
    const state = session.current;
    if (!state.active) return;
    const blocked =
      current.current.blocked ||
      busy.current ||
      !!document.querySelector('[role="dialog"]');
    if (blocked || (appService.native && !state.ready)) {
      setFeedback({
        message: blocked ? "请先完成当前对话框或导入" : "正在匹配拖入文件名…",
        keys: [],
      });
      return;
    }
    const target = landing(state.position);
    const plan = planDrop(state.paths, state.matches, target);
    const destinations = [
      ...new Set(Object.values(plan.targets).map((t) => t.canonical)),
    ];
    const keys = current.current.resources
      .filter((r) => destinations.includes(r.canonical))
      .map((r) => r.key);
    let message = destinations.length
      ? `松开导入到 ${destinations
          .map((cn) => {
            const r = current.current.resources.find((r) => r.canonical === cn);
            return r
              ? `${r.entityName} · ${r.resourceLabel || r.sectionLabel} (${cn})`
              : cn;
          })
          .join("、")}`
      : target
        ? `松开导入到 ${target.canonical}`
        : "松开导入 PNG／APMod，未知 PNG 将选择归属";
    if (plan.unknown.length && destinations.length)
      message += `；另有 ${plan.unknown.length} 个 PNG 需要选择归属`;
    if (plan.packages.length)
      message += `；${plan.packages.length} 个 APMod 将进入包向导`;
    if (plan.unsupported.length)
      message += `；${plan.unsupported.length} 个文件不支持，仅支持 PNG／APMod`;
    if (!appService.native) message += "（浏览器预览，请在桌面版导入）";
    if (current.current.blocked || busy.current) message = "请先完成当前导入";
    setFeedback({
      message,
      keys:
        current.current.blocked || busy.current
          ? []
          : keys.length
            ? keys
            : target
              ? current.current.resources
                  .filter((r) => r.canonical === target.canonical)
                  .map((r) => r.key)
              : [],
    });
  }, [landing]);
  const handler = useCallback(
    (event: DropEvent) => {
      if (event.type === "leave") {
        clear();
        return;
      }
      session.current.position = event.position;
      if (event.type === "enter") {
        clear();
        session.current.active = true;
        session.current.paths = event.paths || [];
        session.current.ready = !appService.native;
        const generation = session.current.generation;
        if (appService.native)
          void appService
            .matchDrop(session.current.paths)
            .then((matches) => {
              if (
                session.current.generation !== generation ||
                !session.current.active
              )
                return;
              session.current.matches = matches;
              session.current.ready = true;
              show();
            })
            .catch(() => {
              /* 松开时重新匹配，悬停失败不猜名称归属。 */
            });
        show();
        return;
      }
      if (event.type === "over") {
        show();
        return;
      }
      const target = landing(event.position);
      const paths = event.paths || [];
      clear();
      if (
        current.current.blocked ||
        busy.current ||
        document.querySelector('[role="dialog"]')
      ) {
        current.current.notice("请先完成当前对话框或导入");
        return;
      }
      if (!appService.native) {
        current.current.notice(
          "请在桌面版拖入本地 PNG／APMod；浏览器预览只展示拖入反馈。",
        );
        return;
      }
      busy.current = true;
      setImporting(true);
      void (async () => {
        try {
          const plan = planDrop(
            paths,
            await appService.matchDrop(paths),
            target,
          );
          const messages: string[] = [];
          if (plan.unsupported.length)
            messages.push(
              `不支持：${plan.unsupported.map((p) => p.split(/[\\/]/).pop()).join("、")}；仅支持 PNG／APMod`,
            );
          if (plan.known.length) {
            try {
              const results = await appService.importDrop(
                plan.known,
                plan.targets,
                abort.current.signal,
              );
              messages.push(dropSummary(results));
              for (const r of results)
                if (r.reason || r.applyError)
                  messages.push(
                    `${r.file}：${r.applyError ? "已入库、未应用：" + r.applyError : r.reason}`,
                  );
            } catch (error) {
              messages.push(`PNG 导入未完成：${String(error)}`);
            }
            await client.invalidateQueries({ queryKey: ["workspace"] });
          }
          if (abort.current.signal.aborted) return;
          if (messages.length) current.current.notice(messages.join("；"));
          if (plan.unknown.length)
            current.current.onUnknown(plan.unknown, plan.packages);
          else if (plan.packages.length)
            current.current.onPackages(plan.packages);
        } catch (error) {
          if (!abort.current.signal.aborted)
            current.current.notice(String(error));
        } finally {
          busy.current = false;
          if (!abort.current.signal.aborted) setImporting(false);
        }
      })();
    },
    [clear, client, landing, show],
  );
  useDrop(handler);
  useEffect(() => {
    clear();
  }, [route, blocked, clear]);
  useEffect(() => {
    abort.current = new AbortController();
    return () => {
      abort.current.abort();
      session.current.generation++;
      session.current.active = false;
    };
  }, []);
  useEffect(() => {
    const elements =
      feedback?.keys.flatMap((key) =>
        [
          ...document.querySelectorAll<HTMLElement>("[data-drop-resource-key]"),
        ].filter((e) => e.dataset.dropResourceKey === key),
      ) || [];
    elements.forEach((e) => e.classList.add("resource-drop-target"));
    return () =>
      elements.forEach((e) => e.classList.remove("resource-drop-target"));
  }, [feedback]);
  return { feedback, importing };
}
