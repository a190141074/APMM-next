// 智能搜索的容错范围、直接优先和稳定身份顺序属于独立可验证的展示逻辑。
import { describe, it, expect } from "vitest";
import { rankSearch, matchesSearch, slotOfResource } from "./resources";
import { classifyImageRow } from "./importClassification";
import {
  candidateName,
  candidateExportName,
  normalizedCandidateInfo,
} from "./candidateInfo";
import type { ResourceRecord } from "../ipc/generated/ResourceRecord";
const row = (key: string, name: string, alias: string[] = [], fileName = "") =>
  ({
    key,
    entityId: "101",
    canonical: "UT_Hero_Card_101",
    fullName: "UT_Hero_Card_101",
    entityName: name,
    aliases: alias,
    files: fileName ? [{ sourceName: fileName, hash: "0123456789abcdef" }] : [],
  }) as unknown as ResourceRecord;
describe("管理搜索", () => {
  it("直接匹配排在相似结果前，同分按完整身份排序", () => {
    const rows = [row("z", "Parunan"), row("b", "Parunan"), row("a", "Paruna")];
    expect(rankSearch(rows, "Paruna").map((x) => [x.row.key, x.score])).toEqual(
      [
        ["a", 0],
        ["b", 0],
        ["z", 0],
      ],
    );
    const result = rankSearch(
      [row("z", "Parunan"), row("a", "Parunan"), row("x", "Parunam")],
      "Parunam",
    );
    expect(result.map((x) => x.row.key)).toEqual(["x", "a", "z"]);
    expect(result[1].score).toBeGreaterThan(0);
  });
  it("昵称与候选名称容错，ID 和哈希不容错", () => {
    const r = row("test", "帕露南", ["Parunan"], "summer-outfit.png");
    expect(matchesSearch(r, "Parunam")).toBe(true);
    expect(matchesSearch(r, "sumer")).toBe(true);
    expect(matchesSearch(r, "Parunam", "id")).toBe(false);
    expect(matchesSearch(r, "UT_Hero_Card_102")).toBe(false);
    expect(matchesSearch(r, "102")).toBe(false);
    expect(matchesSearch(r, "帕")).toBe(true);
    expect(matchesSearch(r, "怕")).toBe(false);
    expect(matchesSearch(r, "0123456789abcdee")).toBe(false);
  });
});
describe("导入归属", () => {
  it("目标槽位决定重复，切回临时仍保留原文件身份", () => {
    const image = {
      id: "source",
      hash: "same",
      path: "fixture.png",
      meta: { sourceName: "原名.png", canonical: "old" },
    };
    const target = {
      entityId: "101",
      section: "card",
      canonical: "UT_Hero_Card_101",
      files: [{ hash: "same" }],
    } as ResourceRecord;
    const classified = classifyImageRow(image, target);
    expect(classified.status).toBe("duplicate");
    expect(classified.target).toEqual({
      heroId: "101",
      sectionKey: "card",
      canonical: target.canonical,
    });
    expect(classified.meta.sourceName).toBe("原名.png");
    const temporary = classifyImageRow(classified, null);
    expect(temporary.target).toEqual({ temporary: true });
    expect(temporary.meta.canonical).toBe("unmatched:same");
    expect(temporary.originalMeta).toEqual(image.meta);
    expect(temporary.id).toBe(image.id);
    expect(temporary.path).toBe(image.path);
    expect(classifyImageRow(image, { ...target, files: [] }).status).toBe(
      "new",
    );
  });
  it.each(["heroes", "cards", "monsters", "chips", "lands", "cursors"])(
    "%s 使用实际分类，不使用库容器分类",
    (category) => {
      expect(
        slotOfResource({
          category,
          entityId: "id",
          section: "part",
          canonical: "slot",
        } as ResourceRecord),
      ).toEqual({
        category,
        heroId: "id",
        sectionKey: "part",
        canonical: "slot",
      });
    },
  );
});
describe("候选信息展示", () => {
  it("只隐藏图片后缀，导出仍为安全 PNG 文件名", () => {
    expect(candidateName("夏日.v2.PNG.disabled")).toBe("夏日.v2");
    expect(candidateName("我的搭配 🌙")).toBe("我的搭配 🌙");
    expect(candidateExportName("我的 / 夏日：搭配 🌙")).toBe(
      "我的  夏日：搭配 🌙.png",
    );
    expect(normalizedCandidateInfo(null).provenance.authorUrls).toEqual([]);
    expect(normalizedCandidateInfo(null).provenance.artistUrls).toEqual([]);
  });
});
