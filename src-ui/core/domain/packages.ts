// 一组分卷只加入队列一次，后端会检查同目录全部分卷及编号。
export function uniquePackages(paths: string[]) {
  const found = new Map<string, string>();
  for (const path of paths) {
    if (!path.toLowerCase().endsWith(".apmod")) continue;
    const key = path
      .replaceAll("\\", "/")
      .toLowerCase()
      .replace(/\.part\d{3,}\.apmod$/, ".apmod");
    if (!found.has(key)) found.set(key, path);
  }
  return [...found.values()];
}
export type ImportTarget = {
  heroId: string;
  sectionKey: string;
  canonical: string;
};
