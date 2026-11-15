// 名称与物理文件身份分离；旧来源记录补齐可选的作者与画师链接列表。
import type { CandidateInfo } from "../ipc/generated/CandidateInfo";
export function candidateName(name: string) {
  return name.replace(/\.(png|webp|jpe?g|gif)(\.disabled)?$/i, "");
}
export function candidateExportName(name: string) {
  return `${
    candidateName(name)
      .replace(/[<>:"/\\|?*\x00-\x1f]/g, "")
      .trim() || "mod"
  }.png`;
}
export function normalizedCandidateInfo(
  info?: CandidateInfo | null,
): CandidateInfo {
  return {
    provenance: {
      author: "",
      authorUrls: [],
      artist: "",
      artistUrls: [],
      sourceUrl: "",
      version: "",
      ...info?.provenance,
    },
    note: info?.note || "",
  };
}
