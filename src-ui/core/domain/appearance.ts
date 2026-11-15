// 配色纯规则：所有输入归一为 sRGB，派生色只通过统一语义变量应用。
import defaults from "../../../config/themes.json";
import type { AppearanceSettings } from "../ipc/generated/AppearanceSettings";
import type { ThemePalette } from "../ipc/generated/ThemePalette";
import type { ThemeScheme } from "../ipc/generated/ThemeScheme";
export type { AppearanceSettings, ThemePalette, ThemeScheme };
export type PaletteMode = "light" | "dark";
export const defaultScheme: ThemeScheme = {
  id: "default",
  name: "APMM 默认",
  ...defaults,
};
export const defaultAppearance: AppearanceSettings = {
  schema: 1,
  mode: "system",
  reduceMotion: false,
  catAnimation: false,
  activeSchemeId: "default",
  schemes: [],
};
export const colorRoles: {
  key: keyof ThemePalette;
  label: string;
  group: string;
}[] = [
  { key: "canvas", label: "页面背景", group: "背景" },
  { key: "sidebar", label: "侧栏背景", group: "背景" },
  { key: "surface", label: "面板背景", group: "背景" },
  { key: "popover", label: "浮层背景", group: "背景" },
  { key: "surfaceAlt", label: "输入与筛选区", group: "背景" },
  { key: "imageBg", label: "图片预览底色", group: "背景" },
  { key: "text", label: "主文字", group: "内容" },
  { key: "muted", label: "辅助文字", group: "内容" },
  { key: "border", label: "边框", group: "内容" },
  { key: "accent", label: "强调色", group: "强调" },
  { key: "onAccent", label: "强调按钮文字", group: "强调" },
  { key: "success", label: "成功", group: "状态" },
  { key: "warning", label: "警告", group: "状态" },
  { key: "danger", label: "错误", group: "状态" },
  { key: "info", label: "信息", group: "状态" },
];
export const validHex = (v: unknown): v is string =>
  typeof v === "string" && /^#[\da-f]{6}$/i.test(v);
export function validateScheme(value: unknown): ThemeScheme {
  if (!value || typeof value !== "object") throw Error("配色文件格式无效");
  const s = value as ThemeScheme;
  if (typeof s.name !== "string" || !s.name.trim() || s.name.length > 80)
    throw Error("名称应为 1–80 个字符");
  for (const mode of ["light", "dark"] as const)
    if (!s[mode] || colorRoles.some(({ key }) => !validHex(s[mode][key])))
      throw Error("颜色必须为 #RRGGBB，且包含完整亮暗配色");
  return {
    id: s.id,
    name: s.name.trim(),
    light: Object.fromEntries(
      colorRoles.map(({ key }) => [key, s.light[key].toLowerCase()]),
    ) as ThemePalette,
    dark: Object.fromEntries(
      colorRoles.map(({ key }) => [key, s.dark[key].toLowerCase()]),
    ) as ThemePalette,
  };
}
export function readAppearance(value: unknown): AppearanceSettings {
  try {
    const a = value as AppearanceSettings;
    if (
      a.schema !== 1 ||
      !["system", "light", "dark"].includes(a.mode) ||
      !Array.isArray(a.schemes) ||
      a.schemes.length > 64
    )
      throw Error();
    const schemes = a.schemes.map(validateScheme);
    const ids = new Set(schemes.map((s) => s.id));
    if (
      ids.size !== schemes.length ||
      ids.has("default") ||
      schemes.some(
        (s) => !/^[\da-f]{8}(?:-[\da-f]{4}){3}-[\da-f]{12}$/i.test(s.id),
      )
    )
      throw Error();
    if (a.activeSchemeId !== "default" && !ids.has(a.activeSchemeId))
      throw Error();
    return {
      schema: 1,
      mode: a.mode,
      reduceMotion: a.reduceMotion === true,
      catAnimation: a.catAnimation === true,
      activeSchemeId: a.activeSchemeId,
      schemes,
    };
  } catch {
    return structuredClone(defaultAppearance);
  }
}
export const selectedScheme = (a: AppearanceSettings) =>
  a.schemes.find((s) => s.id === a.activeSchemeId) || defaultScheme;
export function importScheme(text: string): ThemeScheme {
  if (text.length > 65536) throw Error("配色文件超过 64 KiB");
  const file = JSON.parse(text);
  if (file.schema !== 1 || file.type !== "apmm-theme")
    throw Error("不支持的配色文件版本");
  return { ...validateScheme(file.scheme), id: crypto.randomUUID() };
}
export const exportScheme = (scheme: ThemeScheme) =>
  JSON.stringify({ type: "apmm-theme", schema: 1, scheme }, null, 2);
const clamp = (n: number, max: number) =>
  Math.max(0, Math.min(max, Number.isFinite(n) ? n : 0));
export const hexToRgb = (hex: string) =>
  [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
export const rgbToHex = (rgb: number[]) =>
  "#" +
  rgb
    .map((n) => Math.round(clamp(n, 255)).toString(16).padStart(2, "0"))
    .join("");
export function rgbToCmyk(rgb: number[]) {
  const values = rgb.map((n) => clamp(n, 255) / 255);
  const k = 1 - Math.max(...values);
  return k === 1
    ? [0, 0, 0, 100]
    : [...values.map((n) => ((1 - n - k) / (1 - k)) * 100), k * 100];
}
export const cmykToRgb = (c: number[]) =>
  c
    .slice(0, 3)
    .map((n) => 255 * (1 - clamp(n, 100) / 100) * (1 - clamp(c[3], 100) / 100));
export function rgbToHsv(rgb: number[]) {
  const [r, g, b] = rgb.map((n) => n / 255),
    max = Math.max(r, g, b),
    min = Math.min(r, g, b),
    d = max - min;
  let h =
    d === 0
      ? 0
      : max === r
        ? ((g - b) / d) % 6
        : max === g
          ? (b - r) / d + 2
          : (r - g) / d + 4;
  h = (h * 60 + 360) % 360;
  return [h, max === 0 ? 0 : d / max, max];
}
export function hsvToHex(h: number, s: number, v: number) {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
  };
  return rgbToHex([f(5), f(3), f(1)].map((n) => n * 255));
}
export function contrast(a: string, b: string) {
  const l = (h: string) =>
    hexToRgb(h)
      .map((n) => {
        const c = n / 255;
        return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
      })
      .reduce((sum, n, i) => sum + n * [0.2126, 0.7152, 0.0722][i], 0);
  const x = l(a),
    y = l(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
export function applyPalette(p: ThemePalette, mode: PaletteMode) {
  const root = document.documentElement;
  root.dataset.theme = mode;
  const names: Record<keyof ThemePalette, string> = {
    canvas: "bg",
    surface: "surface",
    surfaceAlt: "surface-alt",
    sidebar: "sidebar",
    popover: "popover",
    imageBg: "image-bg",
    text: "text",
    muted: "muted",
    border: "border",
    accent: "accent",
    onAccent: "on-accent",
    success: "success",
    warning: "warning",
    danger: "danger",
    info: "info",
  };
  for (const { key } of colorRoles)
    root.style.setProperty(`--${names[key]}`, p[key]);
}
