import { describe, expect, it } from "vitest";
import {
  isResourceSize,
  readResourceSize,
  resourceSizeOptions,
  resourceSizeOrder,
  resourceSizes,
} from "./resourcePresentation";
const storage = (values: Record<string, string>) => ({
  getItem: (key: string) => values[key] ?? null,
});

describe("六档资源尺寸偏好", () => {
  it("档位顺序、名称和所有布局尺寸单调递增", () => {
    expect(resourceSizeOptions.map((option) => option.label)).toEqual([
      "特小",
      "小",
      "中",
      "标准",
      "大",
      "特大",
    ]);
    for (const metric of [
      "cardWidth",
      "cardHeight",
      "thumb",
      "heroThumb",
      "heroCardWidth",
      "heroRowHeight",
    ] as const) {
      const values = resourceSizeOrder.map(
        (size) => resourceSizes[size][metric],
      );
      expect(
        values.every(
          (value, index) => index === 0 || value > values[index - 1],
        ),
      ).toBe(true);
    }
  });
  it("默认统一中档，不沿用不同板块的独立值", () => {
    const s = storage({ "apmm.resource-size.heroes": "xlarge" });
    expect(readResourceSize(s, "heroes")).toBe("medium");
    expect(readResourceSize(s, "cards")).toBe("medium");
  });
  it("统一大小读取全局值，取消后读取各板块独立值", () => {
    const values = {
      "apmm.resource-size-global": "standard",
      "apmm.resource-size.heroes": "tiny",
      "apmm.resource-size.cards": "large",
      "apmm.resource-size-unified": "true",
    };
    expect(readResourceSize(storage(values), "heroes")).toBe("standard");
    expect(readResourceSize(storage(values), "cards")).toBe("standard");
    values["apmm.resource-size-unified"] = "false";
    expect(readResourceSize(storage(values), "heroes")).toBe("tiny");
    expect(readResourceSize(storage(values), "cards")).toBe("large");
  });
  it("无效偏好不会索引不存在的尺寸", () => {
    for (const value of ["obsolete", "toString", "__proto__", ""]) {
      expect(isResourceSize(value)).toBe(false);
      expect(
        readResourceSize(
          storage({ "apmm.resource-size-global": value }),
          "heroes",
        ),
      ).toBe("medium");
    }
  });
});
