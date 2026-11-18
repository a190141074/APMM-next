// 图片进入视口后才请求；目录可操作状态不依赖封面完成。
import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { workshopService } from "../../core/services/workshop";
import { Modal } from "../../shared/Ui";
import { PanZoomImage } from "../../shared/PanZoomImage";
export function WorkshopImage({
  id,
  name,
  title,
  revision,
  zoom = false,
}: {
  id: string;
  name: string;
  title: string;
  revision: string | number | bigint;
  zoom?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  const [failed, setFailed] = useState("");
  const [open, setOpen] = useState(false);
  useEffect(() => {
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        setVisible(true);
        observer.disconnect();
      }
    });
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);
  const q = useQuery({
    queryKey: ["workshop-image", id, name, String(revision)],
    queryFn: () => workshopService.image(id, name),
    enabled: visible && !!name,
    staleTime: Infinity,
    retry: false,
  });
  return (
    <>
      <div className="workshop-image" ref={ref}>
        {q.data && failed !== q.data ? (
          zoom ? (
            <button
              className="workshop-zoom"
              aria-label={`放大${title}`}
              onClick={() => setOpen(true)}
            >
              <img
                src={q.data}
                alt={title}
                onError={() => setFailed(q.data || "")}
              />
            </button>
          ) : (
            <img
              src={q.data}
              alt={title}
              onError={() => setFailed(q.data || "")}
            />
          )
        ) : (
          <span>
            {q.isError || failed === q.data ? "封面暂不可用" : "暂无预览"}
          </span>
        )}
      </div>
      {open && (
        <Modal open wide title={title} onClose={() => setOpen(false)}>
          <PanZoomImage path={q.data} label={title} />
        </Modal>
      )}
    </>
  );
}
