// 列表与卡片按可见资源的位置衔接；动画只影响展示，快速切换和卸载均可取消。
import { useLayoutEffect, useRef, useState, type RefObject } from "react";
import { useAppearance } from "./useAppearance";
type Layout = "grid" | "list" | "compact";
const positions = new Map<string, number>();
export function useResourceLayout(
  ref: RefObject<HTMLElement>,
  route: string,
  initialScroll = 0,
) {
  const [layout, setLayout] = useState<Layout>(() => {
    const saved = localStorage.getItem(`apmm.resource-layout.${route}`);
    if (route === "heroes" && (saved === "list" || saved === "compact"))
      return "compact";
    return saved === "grid" || saved === "list"
      ? saved
      : route === "cards"
        ? "list"
        : "grid";
  });
  const { appearance } = useAppearance();
  const rects = useRef(new Map<string, DOMRect>());
  const animations = useRef<Animation[]>([]);
  const frame = useRef(0);
  const generation = useRef(0);
  const pendingScroll = useRef<number | null>(
    positions.get(`${route}.${layout}`) ?? initialScroll,
  );
  const rememberScroll = (top: number) =>
    positions.set(`${route}.${layout}`, top);
  const cancel = () => {
    cancelAnimationFrame(frame.current);
    animations.current.forEach((a) => a.cancel());
    animations.current = [];
  };
  const change = (next: Layout) => {
    if (next === layout) return;
    rects.current.clear();
    const root = ref.current;
    if (root) {
      rememberScroll(root.scrollTop);
      for (const el of root.querySelectorAll<HTMLElement>(
        "[data-resource-view-key]",
      )) {
        const key = el.dataset.resourceViewKey!;
        if (!rects.current.has(key))
          rects.current.set(key, el.getBoundingClientRect());
      }
      root.dataset.layoutSwitching = "true";
    }
    pendingScroll.current = positions.get(`${route}.${next}`) || 0;
    cancel();
    localStorage.setItem(`apmm.resource-layout.${route}`, next);
    setLayout(next);
  };
  useLayoutEffect(() => {
    const root = ref.current;
    if (!root) return;
    // 每种角色布局保留自己的位置；先挂载新布局再恢复，避免被旧高度截断。
    if (pendingScroll.current !== null) {
      root.scrollTop = pendingScroll.current;
      pendingScroll.current = null;
    }
    if (!rects.current.size) {
      delete root.dataset.layoutSwitching;
      return;
    }
    const token = ++generation.current;
    root.dataset.layoutSwitching = "true";
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const stop = () => {
      cancel();
      delete root.dataset.layoutSwitching;
    };
    frame.current = requestAnimationFrame(() => {
      if (!appearance.reduceMotion && !media.matches && !document.hidden) {
        const visible = [
          ...root.querySelectorAll<HTMLElement>("[data-resource-view-key]"),
        ].slice(0, 24);
        for (const el of visible) {
          const from = rects.current.get(el.dataset.resourceViewKey!);
          const to = el.getBoundingClientRect();
          if (!to.width || !to.height) continue;
          const dx = from
            ? Math.max(-90, Math.min(90, from.left - to.left))
            : 0;
          const dy = from ? Math.max(-65, Math.min(65, from.top - to.top)) : 8;
          animations.current.push(
            el.animate(
              [
                {
                  opacity: 0.25,
                  transform: `translate(${dx}px,${dy}px) scale(.97)`,
                },
                { opacity: 1, transform: "none" },
              ],
              { duration: 280, easing: "cubic-bezier(.16,1,.3,1)" },
            ),
          );
        }
      }
      rects.current.clear();
      Promise.allSettled(animations.current.map((a) => a.finished)).then(() => {
        if (generation.current === token) delete root.dataset.layoutSwitching;
      });
    });
    const changed = () => {
      if (media.matches) stop();
    };
    media.addEventListener("change", changed);
    return () => {
      generation.current++;
      stop();
      media.removeEventListener("change", changed);
    };
  }, [layout, appearance.reduceMotion]);
  useLayoutEffect(() => () => cancel(), []);
  return { layout, change, rememberScroll };
}
