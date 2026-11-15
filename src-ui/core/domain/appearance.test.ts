import { describe, expect, it } from "vitest";
import { cmykToRgb, contrast, defaultAppearance, defaultScheme, exportScheme, hexToRgb, hsvToHex, importScheme, readAppearance, rgbToCmyk, rgbToHex, rgbToHsv } from "./appearance";
describe("配色输入与持久边界", () => {
  it("黑白与常见颜色经过 RGB/CMYK/HSV 往返不丢失 sRGB", () => {
    for (const color of ["#000000", "#ffffff", "#64cbb8", "#7360df", "#ff0000"]) {
      const rgb = hexToRgb(color);
      expect(rgbToHex(cmykToRgb(rgbToCmyk(rgb)))).toBe(color);
      const [h, s, v] = rgbToHsv(rgb); expect(hsvToHex(h, s, v)).toBe(color);
    }
    expect(rgbToCmyk([0, 0, 0])).toEqual([0, 0, 0, 100]);
  });
  it("方案导入保留亮暗颜色并分配独立标识", () => {
    const imported = importScheme(exportScheme(defaultScheme));
    expect(imported.id).not.toBe("default");
    expect(imported.light).toEqual(defaultScheme.light); expect(imported.dark).toEqual(defaultScheme.dark);
    expect(readAppearance({ ...defaultAppearance, activeSchemeId: imported.id, schemes: [imported] }).activeSchemeId).toBe(imported.id);
  });
  it("拒绝 CSS 注入、缺失角色、未知版本和悬空选择", () => {
    const broken = structuredClone(defaultScheme); broken.light.accent = "url(https://invalid)";
    expect(() => importScheme(exportScheme(broken))).toThrow();
    expect(() => importScheme('{"schema":2,"type":"apmm-theme"}')).toThrow();
    expect(readAppearance({ ...defaultAppearance, activeSchemeId: "missing" })).toEqual(defaultAppearance);
  });
  it("对比度计算能识别极端不可读输入", () => {
    expect(contrast("#000000", "#ffffff")).toBe(21);
    expect(contrast("#ffffff", "#ffffff")).toBe(1);
  });
});
