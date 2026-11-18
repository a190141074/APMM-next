// 更新检查统一使用分钟，按最后成功或尝试时间计算下一次执行，失败也遵守间隔。
type CheckSettings = {
  updateAutoCheck?: boolean;
  updateIntervalMinutes?: number;
  updateLastCheckedAt?: number | null;
};

export function updateCheckDelay(
  settings: CheckSettings | undefined,
  lastAttempt: number,
  now: number,
): number | null {
  if (!settings || settings.updateAutoCheck === false) return null;
  const value = settings.updateIntervalMinutes;
  const minutes =
    value && Number.isInteger(value) && value >= 1 && value <= 43200
      ? value
      : 60;
  const last = Math.max(settings.updateLastCheckedAt || 0, lastAttempt);
  // 浏览器计时器使用有符号 32 位延时；最长三十天需分段等待，避免溢出后忙循环。
  return Math.min(
    2147483647,
    Math.max(0, (minutes * 60 - (now - last)) * 1000),
  );
}
