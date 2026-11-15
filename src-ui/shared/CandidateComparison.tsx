// 信息弹窗左侧只读对比原图与具体候选；缩放、平移和复位不改变游戏应用状态。
import { useState } from "react";
import { Maximize2, RotateCcw } from "lucide-react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import type { ModFile } from "../core/ipc/generated/ModFile";
import { candidateName } from "../core/domain/candidateInfo";
import { PanZoomImage } from "./PanZoomImage";
import { Badge, Modal } from "./Ui";
export function CandidateComparison({
  row,
  file,
}: {
  row: ResourceRecord;
  file: ModFile;
}) {
  const [reset, setReset] = useState([0, 0]);
  const [zoom, setZoom] = useState<number | null>(null);
  const images = [
    {
      label: "官方原图",
      path: row.originalUrl,
      status: row.originalStatus,
      revision: undefined,
    },
    {
      label: "当前候选",
      path: file.path,
      status: undefined,
      revision: file.hash,
    },
  ];
  return (
    <div className="mod-info-previews">
      {images.map((image, index) => (
        <section className="mod-preview" key={image.label}>
          <div className="mod-preview-heading">
            <strong>{image.label}</strong>
            {index === 1 && (
              <Badge
                tone={
                  row.activeHash === file.hash && !row.disabled ? "success" : ""
                }
              >
                {row.activeHash === file.hash && !row.disabled
                  ? "已应用"
                  : "候选"}
              </Badge>
            )}
            <div className="spacer" />
            <button
              type="button"
              className="icon-button"
              aria-label={`复位${image.label}`}
              onClick={() =>
                setReset(
                  reset.map((value, i) => (i === index ? value + 1 : value)),
                )
              }
            >
              <RotateCcw size={14} />
            </button>
            <button
              type="button"
              className="icon-button"
              aria-label={`放大${image.label}`}
              onClick={() => setZoom(index)}
            >
              <Maximize2 size={14} />
            </button>
          </div>
          <PanZoomImage
            key={`${index}-${reset[index]}`}
            path={image.path}
            label={image.label}
            status={image.status}
            revision={image.revision}
          />
          {index === 1 && <small>{candidateName(file.sourceName)}</small>}
        </section>
      ))}
      {zoom !== null && (
        <Modal
          open
          wide
          onClose={() => setZoom(null)}
          title={images[zoom].label}
        >
          <div className="mod-preview-expanded">
            <PanZoomImage
              path={images[zoom].path}
              label={images[zoom].label}
              status={images[zoom].status}
              revision={images[zoom].revision}
            />
          </div>
        </Modal>
      )}
    </div>
  );
}
