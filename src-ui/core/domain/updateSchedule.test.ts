import { describe, expect, it } from "vitest";
import { updateCheckDelay } from "./updateSchedule";

describe("分钟更新检查调度", () => {
  it("一分钟间隔在六十秒到期，不沿用十五分钟轮询", () => {
    const settings = { updateIntervalMinutes: 1, updateLastCheckedAt: 1000 };
    expect(updateCheckDelay(settings, 0, 1030)).toBe(30000);
    expect(updateCheckDelay(settings, 0, 1060)).toBe(0);
  });
  it("失败尝试同样节流，间隔改变后重新计算剩余时间", () => {
    expect(updateCheckDelay({ updateIntervalMinutes: 2 }, 1000, 1060)).toBe(
      60000,
    );
    expect(updateCheckDelay({ updateIntervalMinutes: 1 }, 1000, 1060)).toBe(0);
  });
  it("关闭后不再调度，缺省使用六十分钟", () => {
    expect(updateCheckDelay({ updateAutoCheck: false }, 0, 10000)).toBeNull();
    expect(updateCheckDelay(undefined, 0, 10000)).toBeNull();
    expect(updateCheckDelay({}, 1000, 1000)).toBe(3600000);
    expect(updateCheckDelay({ updateIntervalMinutes: 0 }, 1000, 1000)).toBe(
      3600000,
    );
    expect(updateCheckDelay({ updateIntervalMinutes: 43200 }, 1000, 1000)).toBe(
      2147483647,
    );
  });
});
