// 预览副本按摘要读取，放大只改变显示，不改写作品资源。
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { authorService } from "../../core/services/author";
import { Modal } from "../../shared/Ui";
import { PanZoomImage } from "../../shared/PanZoomImage";
export function AuthorImage({ hash, label }: { hash: string; label: string }) {
  const q = useQuery({
    queryKey: ["author-image", hash],
    queryFn: () => authorService.image(hash),
    staleTime: Infinity,
    retry: false,
  });
  const [open, setOpen] = useState(false);
  return (
    <>
      <button
        type="button"
        className="author-thumbnail"
        disabled={!q.data}
        aria-label={`放大${label}`}
        onClick={() => setOpen(true)}
      >
        {q.data ? (
          <img src={q.data} alt={label} />
        ) : (
          <span>{q.isError ? "预览不可用" : "载入中"}</span>
        )}
      </button>
      {open && (
        <Modal open wide title={label} onClose={() => setOpen(false)}>
          <PanZoomImage path={q.data} label={label} />
        </Modal>
      )}
    </>
  );
}
