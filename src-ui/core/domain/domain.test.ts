// 验证跨页面异步状态和资源身份边界，避免不同文件被误并入同一项。
import { describe, it, expect } from "vitest";
import { mergeTasks } from "./tasks";
import { uniquePackages } from "./packages";
import { orderedQueue } from "./applyQueue";
import { apxFileState, apxInstallState } from "./apx";
import {
  groupResources,
  filterResources,
  matchesSearch,
  variantGroups,
} from "./resources";
import type { TaskSnapshot } from "../ipc/generated/TaskSnapshot";
import type { ResourceRecord } from "../ipc/generated/ResourceRecord";
describe("工作台规则", () => {
  it("迟到的轮询不能覆盖完成事件", () => {
    const newer = {
      id: "task",
      revision: 4,
      status: "completed",
    } as TaskSnapshot;
    const old = { id: "task", revision: 2, status: "running" } as TaskSnapshot;
    expect(mergeTasks([newer], [old])[0]).toBe(newer);
  });
  it("多分卷只加入一次，并保留其他独立包", () => {
    expect(
      uniquePackages([
        "D:\\a.part001.apmod",
        "d:/a.part002.apmod",
        "D:/b.apmod",
        "x.png",
      ]),
    ).toEqual(["D:\\a.part001.apmod", "D:/b.apmod"]);
  });
  it("同编号地块配对且不合并资源文件", () => {
    const records = [
      { key: "land/1", groupKey: "land:1", files: [1, 2, 3] },
      { key: "platform/1", groupKey: "land:1", files: [] },
    ] as unknown as ResourceRecord[];
    const groups = groupResources(records, "lands");
    expect(groups).toHaveLength(1);
    expect(groups[0].resources).toHaveLength(2);
    expect(groups[0].resources[0].files).toHaveLength(3);
  });
  it("搜索完整官方后缀且过滤 Mod 状态", () => {
    const row = {
      category: "cards",
      entityName: "测试",
      entityId: "20006",
      canonical: "UT_HandCard_20006_sfw",
      section: "hand_card",
      sectionLabel: "手牌",
      files: [],
    } as unknown as ResourceRecord;
    expect(
      filterResources([row], "cards", "20006 sfw", "no-mod", ""),
    ).toHaveLength(1);
    expect(
      filterResources([row], "cards", "20006", "has-mod", ""),
    ).toHaveLength(0);
  });
});

describe("目录与应用契约", () => {
  it("应用队列在失败后继续并保留最后点击顺序", async () => {
    const enqueue = orderedQueue();
    const seen: number[] = [];
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const first = enqueue(async () => {
      await gate;
      seen.push(1);
      throw Error("缺少文件");
    });
    const failed = expect(first).rejects.toThrow("缺少文件");
    const second = enqueue(async () => {
      seen.push(2);
    });
    const third = enqueue(async () => {
      seen.push(3);
    });
    expect(seen).toEqual([]);
    release();
    await failed;
    await second;
    await third;
    expect(seen).toEqual([1, 2, 3]);
  });
  it("智能搜索覆盖别名而 ID 搜索保持资源身份", () => {
    const r = {
      key: "card/UT_Hero_Card_101",
      entityId: "101",
      entityName: "帕露南",
      canonical: "UT_Hero_Card_101",
      aliases: ["Parunan", "小帕"],
      files: [{ sourceName: "custom.png", hash: "abc123" }],
    } as ResourceRecord;
    expect(matchesSearch(r, "小帕")).toBe(true);
    expect(matchesSearch(r, "Parunan")).toBe(true);
    expect(matchesSearch(r, "小帕", "id")).toBe(false);
    expect(matchesSearch(r, "card/UT_Hero_Card_101 abc123", "id")).toBe(true);
  });
  it("皮肤与部位分组保留独立资源和三个候选", () => {
    const rows = [
      {
        key: "a",
        variantKey: "base",
        variantLabel: "原皮",
        variantOrder: 0,
        section: "card",
        canonical: "UT_Hero_Card_101",
        files: [1, 2, 3],
      },
      {
        key: "b",
        variantKey: "01",
        variantLabel: "皮肤1",
        variantOrder: 2,
        section: "card",
        canonical: "UT_Hero_Card_101_01",
        files: [],
      },
      {
        key: "c",
        variantKey: "shared",
        variantLabel: "公共资源",
        variantOrder: 1000,
        section: "account_background",
        canonical: "UT_AccountBackground_101",
        files: [],
      },
    ] as unknown as ResourceRecord[];
    expect(variantGroups(rows).map((g) => g.key)).toEqual([
      "base",
      "01",
      "shared",
    ]);
    expect(
      variantGroups(rows, true).find((g) => g.key === "card")?.rows,
    ).toHaveLength(2);
    expect(variantGroups(rows)[0].rows[0].files).toHaveLength(3);
  });
  it("APX 使用校验结果区分缺失、待更新和部分安装", () => {
    expect(apxFileState({ present: true, installed: false })).toBe("待更新");
    expect(
      apxFileState({
        present: true,
        installed: false,
        validationError: "摘要不匹配",
      }),
    ).toBe("校验失败");
    expect(apxFileState({ present: false, installed: false })).toBe("缺失");
    expect(apxInstallState({ totalFiles: 3, installedFiles: 1 })).toBe(
      "部分安装",
    );
    expect(apxInstallState({ totalFiles: 3, installedFiles: 3 })).toBe(
      "已安装",
    );
  });
});
