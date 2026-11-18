// 四档预览共用尺寸定义，虚拟行高与图片布局必须同步调整。
import type { CSSProperties } from "react";
export const resourceSizes = {
  small: {
    label: "小",
    cardWidth: 165,
    cardHeight: 230,
    thumb: 64,
    detailThumb: 96,
    heroThumb: 44,
    heroCardWidth: 208,
    heroRowHeight: 74,
  },
  medium: {
    label: "中",
    cardWidth: 215,
    cardHeight: 280,
    thumb: 80,
    detailThumb: 126,
    heroThumb: 56,
    heroCardWidth: 242,
    heroRowHeight: 88,
  },
  large: {
    label: "大",
    cardWidth: 270,
    cardHeight: 320,
    thumb: 104,
    detailThumb: 160,
    heroThumb: 72,
    heroCardWidth: 284,
    heroRowHeight: 104,
  },
  xlarge: {
    label: "特大",
    cardWidth: 340,
    cardHeight: 400,
    thumb: 144,
    detailThumb: 208,
    heroThumb: 84,
    heroCardWidth: 336,
    heroRowHeight: 118,
  },
} as const;
export type ResourceSize = keyof typeof resourceSizes;
export const resourceSizeOptions = Object.entries(resourceSizes).map(
  ([value, size]) => ({ value, label: size.label }),
);
export function resourceSizeStyle(size: ResourceSize): CSSProperties {
  const metrics = resourceSizes[size];
  return {
    "--list-thumb": `${metrics.thumb}px`,
    "--detail-thumb": `${metrics.detailThumb}px`,
    "--hero-thumb": `${metrics.heroThumb}px`,
  } as CSSProperties;
}
