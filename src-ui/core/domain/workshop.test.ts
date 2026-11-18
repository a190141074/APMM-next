import { describe, expect, it } from "vitest";
import { filterWorks, workStatus } from "./workshop";
import type { ListedWork } from "../ipc/generated/ListedWork";
const entry = (id: string, patch: Partial<ListedWork> = {}): ListedWork => ({
  installedVersion: null,
  locallyModified: false,
  updateAvailable: false,
  work: {
    id,
    title: id,
    author: "作者",
    artist: "画师甲",
    authorUrls: [],
    artistUrls: [],
    sourceUrl: "",
    description: "独立作品",
    category: "卡面",
    tags: [{ name: "透明", highlight: false }],
    version: "1",
    files: [],
    previews: [],
    history: [],
    cover: "",
    coverDataUrl: null,
    publishedId: null,
    gameVersion: "",
    changeNote: "",
    updatedAt: 0,
    revision: 0,
    archive: "a.zip",
    sha256: "0".repeat(64),
    bytes: 1,
  },
  ...patch,
});
describe("工坊状态与筛选", () => {
  it("区分首次获取、同版本、内容更新和本地修改", () => {
    expect(workStatus(entry("new"))).toBe("new");
    expect(workStatus(entry("same", { installedVersion: "1" }))).toBe(
      "installed",
    );
    expect(
      workStatus(
        entry("changed", { installedVersion: "1", updateAvailable: true }),
      ),
    ).toBe("update");
    expect(
      workStatus(
        entry("edited", { installedVersion: "1", locallyModified: true }),
      ),
    ).toBe("modified");
  });
  it("标题介绍作者标签参与搜索，无历史时间的排序稳定", () => {
    const items = [entry("乙"), entry("甲")];
    expect(filterWorks(items, "画师甲", "", "", "updated")).toHaveLength(2);
    expect(
      filterWorks(items, "作者 透明", "卡面", "new", "updated").map(
        (i) => i.work.id,
      ),
    ).toEqual(["乙", "甲"]);
    expect(
      filterWorks(items, "独立", "", "", "name").map((i) => i.work.id),
    ).toEqual(["甲", "乙"]);
    expect(filterWorks(items, "不存在", "", "", "updated")).toEqual([]);
  });
});
