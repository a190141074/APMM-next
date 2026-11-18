// 六档预览共用尺寸定义；列表与详情的候选卡必须使用相同的实际边长。
import type { CSSProperties } from "react";
export const resourceSizes = {
  tiny: {
    label: "特小",
    cardWidth: 140,
    cardHeight: 196,
    thumb: 52,
    heroThumb: 40,
    heroCardWidth: 190,
    heroRowHeight: 68,
  },
  small: {
    label: "小",
    cardWidth: 165,
    cardHeight: 230,
    thumb: 64,
    heroThumb: 44,
    heroCardWidth: 208,
    heroRowHeight: 74,
  },
  medium: {
    label: "中",
    cardWidth: 205,
    cardHeight: 265,
    thumb: 76,
    heroThumb: 52,
    heroCardWidth: 228,
    heroRowHeight: 82,
  },
  standard: {
    label: "标准",
    cardWidth: 235,
    cardHeight: 295,
    thumb: 88,
    heroThumb: 60,
    heroCardWidth: 252,
    heroRowHeight: 92,
  },
  large: {
    label: "大",
    cardWidth: 285,
    cardHeight: 340,
    thumb: 112,
    heroThumb: 76,
    heroCardWidth: 292,
    heroRowHeight: 106,
  },
  xlarge: {
    label: "特大",
    cardWidth: 340,
    cardHeight: 400,
    thumb: 144,
    heroThumb: 84,
    heroCardWidth: 336,
    heroRowHeight: 118,
  },
} as const;
export type ResourceSize = keyof typeof resourceSizes;
export const resourceSizeOrder = [
  "tiny",
  "small",
  "medium",
  "standard",
  "large",
  "xlarge",
] as const satisfies readonly ResourceSize[];
export const resourceSizeOptions = Object.entries(resourceSizes).map(
  ([value, size]) => ({ value, label: size.label }),
);
export function isResourceSize(
  value: string | null | undefined,
): value is ResourceSize {
  return !!value && Object.hasOwn(resourceSizes, value);
}
/** 统一模式只读全局档位，独立模式保留各板块偏好；无效旧值回退到中档。 */
export function readResourceSize(
  storage: Pick<Storage, "getItem">,
  route: string,
): ResourceSize {
  const key =
    storage.getItem("apmm.resource-size-unified") !== "false"
      ? "apmm.resource-size-global"
      : "apmm.resource-size." + route;
  const value = storage.getItem(key);
  return isResourceSize(value) ? value : "medium";
}
export function resourceSizeStyle(size: ResourceSize): CSSProperties {
  const metrics = resourceSizes[size];
  return {
    "--candidate-thumb": `${metrics.thumb}px`,
    "--hero-thumb": `${metrics.heroThumb}px`,
  } as CSSProperties;
}
