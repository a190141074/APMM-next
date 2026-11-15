// 参考 NapCatQQ-Desktop 的 WAAPI 页面过渡：短退场后换页，反向切换接续当前帧。
// 本实现按 APMM 路由和低位移节奏重写；请求序号阻止旧动画提交过期页面。
import { useLayoutEffect, useRef, useState } from "react";
import type { Route } from "../core/domain/resources";
import { useAppearance } from "./useAppearance";
export function usePageNavigation(order: readonly Route[]) {
  const [route, display] = useState<Route>("overview"),
    [target, setRoute] = useState<Route>("overview");
  const pageRef = useRef<HTMLElement>(null),
    animation = useRef<Animation | null>(null);
  const shown = useRef(route),
    generation = useRef(0),
    direction = useRef(1),
    entering = useRef(false);
  shown.current = route;
  const { appearance } = useAppearance();
  const [systemReduced, setSystemReduced] = useState(
    () => matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  useLayoutEffect(() => {
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const changed = () => setSystemReduced(media.matches);
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  }, []);
  const reduced = appearance.reduceMotion || systemReduced;
  useLayoutEffect(() => {
    const el = pageRef.current;
    if (!el) return;
    const token = ++generation.current;
    const current =
      animation.current?.playState === "running" ? getComputedStyle(el) : null;
    const from = current
      ? { opacity: current.opacity, transform: current.transform }
      : { opacity: 1, transform: "none" };
    animation.current?.cancel();
    animation.current = null;
    el.style.pointerEvents = "";
    if (reduced || !el.animate || document.hidden) {
      entering.current = false;
      display(target);
      return;
    }
    if (target === shown.current) {
      if (current)
        animation.current = el.animate(
          [from, { opacity: 1, transform: "none" }],
          { duration: 160, easing: "cubic-bezier(.16,1,.3,1)" },
        );
      return;
    }
    direction.current =
      order.indexOf(target) >= order.indexOf(shown.current) ? 1 : -1;
    el.style.pointerEvents = "none";
    const exit = el.animate(
      [
        from,
        {
          opacity: 0,
          transform: `translate(${-direction.current * 6}px, -3px)`,
        },
      ],
      { duration: 75, easing: "ease-in", fill: "forwards" },
    );
    animation.current = exit;
    exit.onfinish = () => {
      if (generation.current === token) {
        entering.current = true;
        el.scrollTop = 0;
        display(target);
      }
    };
  }, [target, reduced]);
  useLayoutEffect(() => {
    const el = pageRef.current;
    if (!el || !entering.current) return;
    entering.current = false;
    animation.current?.cancel();
    el.style.pointerEvents = "";
    if (!reduced)
      animation.current = el.animate(
        [
          {
            opacity: 0,
            transform: `translate(${direction.current * 6}px, 4px)`,
          },
          { opacity: 1, transform: "none" },
        ],
        {
          duration: 165,
          easing: "cubic-bezier(.16,1,.3,1)",
          fill: "backwards",
        },
      );
  }, [route, reduced]);
  useLayoutEffect(
    () => () => {
      generation.current++;
      animation.current?.cancel();
    },
    [],
  );
  return { route, target, setRoute, pageRef };
}
