import * as Dialog from "@radix-ui/react-dialog";
import { X, ImageOff, LoaderCircle } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useImage } from "../hooks/useApp";
import catAnimation from "./cat-loading.gif";
export function Modal({
  open,
  onClose,
  title,
  children,
  wide = false,
  className = "",
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  wide?: boolean;
  className?: string;
}) {
  return (
    <Dialog.Root open={open} onOpenChange={(v) => !v && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="modal-overlay" />
        <Dialog.Content className={`modal ${wide ? "wide" : ""} ${className}`}>
          <header>
            <Dialog.Title>{title}</Dialog.Title>
            <Dialog.Close className="icon-button" aria-label="关闭">
              <X size={18} />
            </Dialog.Close>
          </header>
          <Dialog.Description className="sr-only">
            {title}的内容和操作
          </Dialog.Description>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
export function AssetImage({
  path,
  label,
  className = "",
  thumbnail = false,
  revision,
  status,
}: {
  path?: string | null;
  label: string;
  className?: string;
  thumbnail?: boolean;
  revision?: string | null;
  status?: string;
}) {
  const image = useImage();
  const src = image(path, thumbnail, revision);
  const [failed, setFailed] = useState("");
  return (
    <div className={`asset-image ${className}`}>
      {path && failed !== src ? (
        <img
          loading="lazy"
          src={src}
          alt={label}
          onError={() => setFailed(src)}
        />
      ) : (
        <div className="image-empty">
          <ImageOff size={25} />
          <span>
            {path && failed === src
              ? "图片加载失败"
              : status === "missing"
                ? "原图文件缺失"
                : status === "invalid"
                  ? "原图校验失败"
                  : status === "not-extracted"
                    ? "原图尚未提取"
                    : "暂无预览"}
          </span>
        </div>
      )}
    </div>
  );
}
export function Empty({
  title,
  detail,
  children,
}: {
  title: string;
  detail?: string;
  children?: ReactNode;
}) {
  return (
    <div className="empty">
      <ImageOff size={34} />
      <h3>{title}</h3>
      <p>{detail}</p>
      {children}
    </div>
  );
}
export function Loading() {
  const cat =
    localStorage.getItem("apmm.cat-mode") === "true" &&
    localStorage.getItem("apmm.reduce-motion") !== "true" &&
    !matchMedia("(prefers-reduced-motion: reduce)").matches;
  return (
    <div className="empty">
      {cat ? (
        <img src={catAnimation} className="cat-loading" alt="加载中" />
      ) : (
        <LoaderCircle className="spin" />
      )}
      <p>正在读取工作区…</p>
    </div>
  );
}
export function Badge({
  children,
  tone = "",
}: {
  children: ReactNode;
  tone?: string;
}) {
  return <span className={`badge ${tone}`}>{children}</span>;
}
