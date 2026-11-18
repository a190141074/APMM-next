// 详情和直接展开列表共享候选操作；应用仍经既有 IPC，持久状态来自工作区快照。
import { useState } from "react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import type { ModFile } from "../core/ipc/generated/ModFile";
import { useActions } from "./useApp";
import { Modal } from "../shared/Ui";
import { PackageDialog } from "../modules/PackageDialog";
import { ResourceFileActions } from "../modules/ResourceFileActions";
import { PanZoomImage } from "../shared/PanZoomImage";
export function useResourceActions(resources: ResourceRecord[]) {
  const [zoom, setZoom] = useState<{
    row: ResourceRecord;
    file?: ModFile;
  } | null>(null);
  const [manage, setManage] = useState<{
    row: ResourceRecord;
    file: ModFile;
  } | null>(null);
  const [importing, setImporting] = useState<ResourceRecord | null>(null);
  const [choices, setChoices] = useState<Record<string, string>>({});
  const [pending, setPending] = useState(0);
  const { api, act, notice, setNotice } = useActions();
  const zoomFile = zoom?.file
    ? resources
        .find((r) => r.key === zoom.row.key)
        ?.files.find(
          (f) => f.hash === zoom.file?.hash || f.path === zoom.file?.path,
        ) || zoom.file
    : undefined;
  const apply = async (rows: ResourceRecord[], hash?: string | null) => {
    setPending((n) => n + 1);
    await act(
      () =>
        api.apply(
          rows.map((r) => ({
            entityId: r.entityId,
            section: r.section,
            canonical: r.canonical,
            hash: hash === undefined ? choices[r.key] || r.activeHash : hash,
          })),
        ),
      "应用状态已刷新",
    );
    setPending((n) => n - 1);
  };
  return {
    api,
    act,
    notice,
    setNotice,
    pending,
    choices,
    setChoices,
    setZoom,
    setManage,
    setImporting,
    apply,
    dialogs: (
      <>
        {importing && (
          <PackageDialog
            mode="import"
            importKind="images"
            target={{
              heroId: importing.entityId,
              sectionKey: importing.section,
              canonical: importing.canonical,
            }}
            onClose={() => setImporting(null)}
          />
        )}
        {manage && (
          <ResourceFileActions {...manage} onClose={() => setManage(null)} />
        )}
        {zoom && (
          <Modal
            open
            onClose={() => setZoom(null)}
            title={`${zoom.row.entityName} · ${zoom.row.variantLabel} · ${zoom.row.resourceLabel}`}
            wide
          >
            <div className="comparison">
              <div>
                <p>官方原图</p>
                <PanZoomImage
                  path={zoom.row.originalUrl}
                  status={zoom.row.originalStatus}
                  label={zoom.row.canonical}
                />
              </div>
              {zoomFile && (
                <div>
                  <p>{zoomFile.sourceName}</p>
                  <PanZoomImage
                    path={zoomFile.path}
                    label={zoomFile.sourceName}
                    revision={zoomFile.hash}
                  />
                </div>
              )}
            </div>
          </Modal>
        )}
      </>
    ),
  };
}
