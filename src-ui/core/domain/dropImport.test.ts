import { describe, expect, it } from "vitest";
import { dropSummary, planDrop } from "./dropImport";
const rule = {
  category: "cursors",
  heroId: "cursor",
  sectionKey: "cursor",
  canonical: "cursor_up",
};
const landing = {
  heroId: "cursor",
  sectionKey: "cursor",
  canonical: "cursor_down",
};
describe("拖入分流", () => {
  it("规则优先于错误落点，剩余 PNG 直接归属落点", () => {
    const plan = planDrop(
      ["cursor_up.png", "unknown.png"],
      [{ path: "cursor_up.png", target: rule }],
      landing,
    );
    expect(plan.targets["cursor_up.png"].canonical).toBe("cursor_up");
    expect(plan.targets["unknown.png"].canonical).toBe("cursor_down");
    expect(plan.unknown).toEqual([]);
  });
  it("只有未知 PNG 需要选择，包分卷去重且 JPG 明确不支持", () => {
    const plan = planDrop(
      [
        "cursor_up.PNG",
        "unknown.png",
        "one.part001.apmod",
        "one.part002.apmod",
        "cursor_up.jpg",
      ],
      [{ path: "cursor_up.PNG", target: rule }],
    );
    expect(plan.known).toEqual(["cursor_up.PNG"]);
    expect(plan.unknown).toEqual(["unknown.png"]);
    expect(plan.packages).toEqual(["one.part001.apmod"]);
    expect(plan.unsupported).toEqual(["cursor_up.jpg"]);
  });
  it("Windows 大小写与分隔符不同的同一文件只提交一次", () => {
    expect(
      planDrop(["B:/one.PNG", "b:\\one.png"], [], landing).known,
    ).toHaveLength(1);
  });
  it("入库成功与应用失败分别计数", () => {
    expect(
      dropSummary([
        { file: "a", ok: true, duplicate: false, applied: true },
        {
          file: "b",
          ok: true,
          duplicate: false,
          applied: false,
          applyError: "未配置",
        },
        { file: "c", ok: true, duplicate: true, applied: false },
        {
          file: "d",
          ok: false,
          duplicate: false,
          applied: false,
          reason: "坏图",
        },
      ]),
    ).toBe("新入库 2 · 重复 1 · 导入失败 1 · 已应用 1 · 应用失败 1");
  });
});
