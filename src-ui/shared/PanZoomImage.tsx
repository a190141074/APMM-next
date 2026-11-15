// 图片对比只改变显示变换；滚轮以指针位置缩放，关闭或更换图片时释放监听与拖动状态。
import { useEffect, useRef, useState } from "react";
import { AssetImage } from "./Ui";
const initial = { scale: 1, x: 0, y: 0 };
export function PanZoomImage({
  path,
  label,
  status,
  revision,
}: {
  path?: string | null;
  label: string;
  status?: string;
  revision?: string | null;
}) {
  const viewport = useRef<HTMLDivElement>(null);
  const view = useRef(initial);
  const drag = useRef<{ id: number; x: number; y: number } | null>(null);
  const [transform, setTransform] = useState(initial);
  const [dragging, setDragging] = useState(false);
  const update = (next: typeof initial) => {
    view.current = next;
    setTransform(next);
  };
  const reset = () => {
    update(initial);
    drag.current = null;
    setDragging(false);
  };
  useEffect(() => {
    reset();
    const element = viewport.current;
    if (!element) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const bounds = element.getBoundingClientRect();
      const current = view.current;
      const delta =
        event.deltaY *
        (event.deltaMode === 1
          ? 16
          : event.deltaMode === 2
            ? bounds.height
            : 1);
      const scale = Math.max(
        0.25,
        Math.min(8, current.scale * Math.exp(-delta * 0.002)),
      );
      const ratio = scale / current.scale;
      const x = event.clientX - bounds.left - bounds.width / 2;
      const y = event.clientY - bounds.top - bounds.height / 2;
      update({
        scale,
        x: x - (x - current.x) * ratio,
        y: y - (y - current.y) * ratio,
      });
    };
    element.addEventListener("wheel", wheel, { passive: false });
    return () => {
      element.removeEventListener("wheel", wheel);
      drag.current = null;
    };
  }, [path, revision]);
  return (
    <div
      ref={viewport}
      className={`pan-zoom-image ${dragging ? "dragging" : ""}`}
      role="region"
      aria-label={`${label} 图片预览`}
      tabIndex={0}
      onDragStart={(e) => e.preventDefault()}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        e.currentTarget.focus();
        e.currentTarget.setPointerCapture(e.pointerId);
        drag.current = { id: e.pointerId, x: e.clientX, y: e.clientY };
        setDragging(true);
      }}
      onPointerMove={(e) => {
        const previous = drag.current;
        if (!previous || previous.id !== e.pointerId) return;
        update({
          ...view.current,
          x: view.current.x + e.clientX - previous.x,
          y: view.current.y + e.clientY - previous.y,
        });
        drag.current = { id: e.pointerId, x: e.clientX, y: e.clientY };
      }}
      onPointerUp={(e) => {
        if (drag.current?.id === e.pointerId) {
          drag.current = null;
          setDragging(false);
          if (e.currentTarget.hasPointerCapture(e.pointerId))
            e.currentTarget.releasePointerCapture(e.pointerId);
        }
      }}
      onPointerCancel={() => {
        drag.current = null;
        setDragging(false);
      }}
      onLostPointerCapture={() => {
        drag.current = null;
        setDragging(false);
      }}
      onDoubleClick={(e) => {
        e.preventDefault();
        reset();
      }}
      onKeyDown={(e) => {
        const current = view.current;
        if (e.key === "Home" || e.key === "0") {
          e.preventDefault();
          reset();
        } else if (["+", "=", "-"].includes(e.key)) {
          e.preventDefault();
          update({
            ...current,
            scale: Math.max(
              0.25,
              Math.min(8, current.scale * (e.key === "-" ? 0.8 : 1.25)),
            ),
          });
        } else if (e.key.startsWith("Arrow")) {
          e.preventDefault();
          update({
            ...current,
            x:
              current.x +
              (e.key === "ArrowLeft" ? -20 : e.key === "ArrowRight" ? 20 : 0),
            y:
              current.y +
              (e.key === "ArrowUp" ? -20 : e.key === "ArrowDown" ? 20 : 0),
          });
        }
      }}
    >
      <div
        className="pan-zoom-content"
        style={{
          transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.scale})`,
        }}
      >
        <AssetImage
          path={path}
          label={label}
          status={status}
          revision={revision}
        />
      </div>
      <span className="pan-zoom-hint">滚轮缩放 · 拖动平移 · 双击复位</span>
      <span className="pan-zoom-scale">
        {Math.round(transform.scale * 100)}%
      </span>
    </div>
  );
}
