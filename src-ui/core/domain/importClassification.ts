// 导入向导的归属预览保持原文件身份，只更新目标槽位和该槽位的重复判断。
import type { ResourceRecord } from "../ipc/generated/ResourceRecord";
export function classifyImageRow(image: any, row: ResourceRecord | null) {
  const original = image.originalMeta || image.meta;
  const meta = row
    ? {
        ...image.meta,
        entityId: row.entityId,
        sectionKey: row.section,
        canonical: row.canonical,
      }
    : {
        ...original,
        entityId: `temporary:${image.hash}`,
        sectionKey: "temporary",
        canonical: `unmatched:${image.hash}`,
      };
  const duplicate = !!row?.files.some((file) => file.hash === image.hash);
  return {
    ...image,
    originalMeta: original,
    meta,
    target: row
      ? {
          heroId: row.entityId,
          sectionKey: row.section,
          canonical: row.canonical,
        }
      : { temporary: true },
    status: duplicate ? "duplicate" : "new",
  };
}
