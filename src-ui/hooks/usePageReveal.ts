// 参考 NapCat 的逐项入场行为，使用现有 WAAPI；新节点只播放一次，轮询不重播。
// 页面切换、减少动画和卸载都会取消动画，基础样式始终保持内容可见。
import { useLayoutEffect, type RefObject } from "react";
import { useAppearance } from "./useAppearance";
const selectors =
  ".page-heading, .overview-hero, .overview-version-card, .settings-card, .stat, .quick-card, .workshop-card, .resource-card, .compact-part, .author-file, .empty, .segmented-scroll";

export function usePageReveal(ref: RefObject<HTMLElement>, route: string) {
  const { appearance } = useAppearance();
  useLayoutEffect(() => {
    const root = ref.current;
    if (!root) return;
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const seen = new WeakSet<Element>();
    const animations = new Set<Animation>();
    let frame = 0;
    let resourceBatchShown = false;
    const cancel = () => {
      for (const animation of animations) animation.cancel();
      animations.clear();
    };
    const reveal = () => {
      frame = 0;
      const pending = [...root.querySelectorAll<HTMLElement>(selectors)].filter(
        (el) => {
          if (seen.has(el)) return false;
          seen.add(el);
          if (el.closest('[data-layout-switching="true"]')) return false;
          // 嵌套卡片由最近的整体容器呈现，避免父子叠加位移。
          const parent = el.parentElement?.closest(selectors);
          return (
            !(parent && root.contains(parent)) &&
            !(
              (el.classList.contains("resource-card") ||
                el.classList.contains("compact-part")) &&
              resourceBatchShown
            )
          );
        },
      );
      if (
        pending.some(
          (el) =>
            el.classList.contains("resource-card") ||
            el.classList.contains("compact-part"),
        )
      )
        resourceBatchShown = true;
      if (appearance.reduceMotion || media.matches || document.hidden) return;
      pending.slice(0, 24).forEach((el, index) => {
        if (!el.animate) return;
        const animation = el.animate(
          [
            { opacity: 0, transform: "translateY(6px) scale(.99)" },
            { opacity: 1, transform: "none" },
          ],
          {
            duration: 230,
            delay: Math.min(index, 6) * 30,
            easing: "cubic-bezier(.16,1,.3,1)",
            fill: "backwards",
          },
        );
        animations.add(animation);
        animation.finished
          .catch(() => {})
          .finally(() => animations.delete(animation));
      });
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(reveal);
    };
    const observer = new MutationObserver(schedule);
    observer.observe(root, { childList: true, subtree: true });
    const changed = () => {
      if (media.matches) cancel();
    };
    media.addEventListener("change", changed);
    schedule();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
      cancel();
      media.removeEventListener("change", changed);
    };
  }, [ref, route, appearance.reduceMotion]);
}
