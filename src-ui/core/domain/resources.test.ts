// 混合部位、禁用候选和配对资源的回归用例，验证筛选不改变实体身份。
import { describe, expect, it } from "vitest";
import type { ResourceRecord } from "../ipc/generated/ResourceRecord";
import {
  filterResourceGroups,
  filterResources,
  resourceGroupSummary,
} from "./resources";
const row = (
  key: string,
  entityId: string,
  patch: Partial<ResourceRecord> = {},
): ResourceRecord =>
  ({
    key,
    category: "heroes",
    entityId,
    entityName: `角色${entityId}`,
    canonical: key,
    fullName: key,
    aliases: [],
    section: "body",
    sectionLabel: "身体",
    files: [],
    activeHash: null,
    disabled: false,
    ...patch,
  }) as ResourceRecord;
const file = { hash: "a", sourceName: "候选.png", path: "a.png" };
const mixed = [
  row("cover", "101", { section: "card" }),
  row("body", "101", { files: [file], activeHash: "a" }),
  row("empty", "102"),
  row("disabled", "103", { files: [file], activeHash: "a", disabled: true }),
];
describe("资源页状态与实体边界", () => {
  it("无 Mod 只显示完全没有候选的角色", () => {
    expect(
      filterResourceGroups(mixed, "heroes", "", "no-mod", "").map((g) => g.key),
    ).toEqual(["102"]);
    expect(
      filterResourceGroups(mixed, "heroes", "", "has-mod", "").map(
        (g) => g.key,
      ),
    ).toEqual(["101", "103"]);
  });
  it("已应用排除保留 activeHash 的禁用资源，禁用候选仍属于有 Mod", () => {
    expect(
      filterResources(mixed, "heroes", "", "applied", "").map((r) => r.key),
    ).toEqual(["body"]);
    expect(
      filterResourceGroups(mixed, "heroes", "", "applied", "").map(
        (g) => g.key,
      ),
    ).toEqual(["101"]);
  });
  it("搜索和部位筛选保留整名角色的封面及统计", () => {
    const group = filterResourceGroups(
      mixed,
      "heroes",
      "body",
      "has-mod",
      "body",
      "id",
    )[0];
    expect(group.resources.map((r) => r.key)).toEqual(["cover", "body"]);
    expect(resourceGroupSummary(group)).toEqual({
      cover: mixed[0],
      mods: 1,
      applied: 1,
    });
    expect(mixed[0].files).toHaveLength(0);
  });
  it("角色整体状态与详情逐部位筛选可以得出空详情", () => {
    const group = filterResourceGroups(
      mixed,
      "heroes",
      "",
      "applied",
      "card",
    )[0];
    expect(group.key).toBe("101");
    expect(
      filterResources(group.resources, "heroes", "", "applied", "card"),
    ).toEqual([]);
    expect(
      filterResources(group.resources, "heroes", "", "all", ""),
    ).toHaveLength(2);
  });
  it("地块配对保持完整身份和统计，详情仍逐资源筛选", () => {
    const lands = [
      row("land", "1", {
        category: "lands",
        groupKey: "land:1",
        files: [file],
      }),
      row("platform", "1", { category: "lands", groupKey: "land:1" }),
    ];
    const group = filterResourceGroups(lands, "lands", "", "no-mod", "")[0];
    expect(group.resources).toEqual(lands);
    expect(resourceGroupSummary(group).mods).toBe(1);
    expect(
      filterResources(group.resources, "lands", "", "no-mod", "").map(
        (r) => r.key,
      ),
    ).toEqual(["platform"]);
  });
  it("智能搜索继续按匹配优先级排序实体", () => {
    const rows = [
      row("a", "101", { entityName: "Parunan" }),
      row("b", "102", { entityName: "Parunanx" }),
    ];
    expect(
      filterResourceGroups(rows, "heroes", "Parunanx", "all", "").map(
        (g) => g.key,
      ),
    ).toEqual(["102", "101"]);
  });
});
