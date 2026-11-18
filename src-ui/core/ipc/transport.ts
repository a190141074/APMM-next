// 原生 API 只在传输层使用，浏览器预览由 service 明确选择 mock。
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { DropEvent } from "../domain/dropImport";
export const native = "__TAURI_INTERNALS__" in window;
export { invoke, listen };
export function imageUrl(
  path: string | null | undefined,
  thumbnail = false,
  revision?: string | null,
) {
  if (!path) return "";
  const url =
    path.startsWith("/") || path.startsWith("data:")
      ? path
      : native
        ? convertFileSrc(path, thumbnail ? "thumb" : "vault")
        : path;
  // 内容修订改变请求地址，使同路径外部编辑也能刷新已挂载的图片。
  return revision && !url.startsWith("data:")
    ? `${url}${url.includes("?") ? "&" : "?"}revision=${encodeURIComponent(revision)}`
    : url;
}
export async function pickDirectory() {
  return native ? await open({ directory: true, multiple: false }) : null;
}
export async function pickFiles(
  kind:
    | "images"
    | "packages"
    | "plugins"
    | "author"
    | "credentials"
    | "all" = "all",
) {
  return native
    ? await open({
        multiple: true,
        filters: [
          {
            name: "资源文件",
            extensions:
              kind === "credentials"
                ? ["apmmkey"]
                : kind === "author"
                  ? ["png", "zip", "apmod"]
                  : kind === "plugins"
                    ? ["zip"]
                    : kind === "images"
                      ? ["png"]
                      : kind === "packages"
                        ? ["apmod"]
                        : ["png", "apmod"],
          },
        ],
      })
    : null;
}
export async function saveFile(name: string) {
  return native ? await save({ defaultPath: name }) : null;
}
export async function windowAction(
  action: "minimize" | "toggleMaximize" | "close" | "startDragging",
) {
  if (native) await getCurrentWindow()[action]();
}
export async function fileDrop(callback: (event: DropEvent) => void) {
  if (native)
    return getCurrentWindow().onDragDropEvent((e) => {
      const payload = e.payload;
      callback({
        type: payload.type,
        paths: "paths" in payload ? payload.paths : undefined,
        position:
          "position" in payload
            ? {
                x: payload.position.x / window.devicePixelRatio,
                y: payload.position.y / window.devicePixelRatio,
              }
            : undefined,
      });
    });
  let depth = 0;
  const handler = (event: DragEvent) => {
    if (!event.dataTransfer?.types.includes("Files")) return;
    event.preventDefault();
    if (event.type === "dragenter") depth++;
    if (event.type === "dragleave" && --depth > 0) return;
    if (event.type === "dragleave" || event.type === "drop") depth = 0;
    callback({
      type:
        event.type === "dragenter"
          ? "enter"
          : event.type === "dragover"
            ? "over"
            : event.type === "dragleave"
              ? "leave"
              : "drop",
      paths:
        event.type === "drop"
          ? [...event.dataTransfer.files].map((file) => file.name)
          : undefined,
      position: { x: event.clientX, y: event.clientY },
    });
  };
  for (const type of ["dragenter", "dragover", "dragleave", "drop"])
    window.addEventListener(type, handler as EventListener);
  return () => {
    for (const type of ["dragenter", "dragover", "dragleave", "drop"])
      window.removeEventListener(type, handler as EventListener);
  };
}
