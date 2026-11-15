// 文件导出与屏幕取色的原生边界；浏览器预览保留标准 Web API 降级。
import { appService } from "./app";
import { IPC } from "../ipc/commands";
import { exportScheme, type ThemeScheme } from "../domain/appearance";

export async function saveSchemeFile(scheme: ThemeScheme) {
  const name = `${scheme.name.replace(/[<>:"/\\|?*]/g, "-")}.apmm-theme.json`;
  if (appService.native) {
    const destination = await appService.saveFile(name);
    if (destination)
      await appService.command(IPC.theme_export, { destination, scheme });
    return !!destination;
  }
  const url = URL.createObjectURL(
    new Blob([exportScheme(scheme)], { type: "application/json" }),
  );
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return true;
}
export async function pickScreenColor(): Promise<string | null> {
  if (appService.native) return appService.command(IPC.screen_pick_color);
  const Picker = (
    window as unknown as {
      EyeDropper?: new () => { open: () => Promise<{ sRGBHex: string }> };
    }
  ).EyeDropper;
  if (!Picker)
    throw Error(
      "此浏览器不支持屏幕取色，请使用桌面版，或通过调色盘和数值输入配色。",
    );
  try {
    return (await new Picker().open()).sRGBHex;
  } catch (e) {
    if ((e as Error).name === "AbortError") return null;
    throw e;
  }
}
