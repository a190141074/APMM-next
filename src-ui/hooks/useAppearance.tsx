// 资源库拥有已保存配色；此 Provider 只持有试色草稿与可丢弃的首屏缓存。
import {
  createContext,
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useWorkspace } from "./useApp";
import { appService } from "../core/services/app";
import {
  applyPalette,
  defaultAppearance,
  readAppearance,
  selectedScheme,
  type AppearanceSettings,
  type PaletteMode,
  type ThemeScheme,
} from "../core/domain/appearance";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
type Preview = { scheme: ThemeScheme; mode: PaletteMode } | null;
const Context = createContext<{
  appearance: AppearanceSettings;
  save: (a: AppearanceSettings) => Promise<void>;
  preview: (p: Preview) => void;
  mode: string;
  setMode: (m: string) => void;
  error: string;
}>({
  appearance: defaultAppearance,
  save: async () => {},
  preview: () => {},
  mode: "system",
  setMode: () => {},
  error: "",
});
export const useAppearance = () => useContext(Context);
export function AppearanceProvider({ children }: { children: ReactNode }) {
  const workspace = useWorkspace(),
    client = useQueryClient();
  const [appearance, setAppearance] = useState(() => {
    try {
      return readAppearance(
        JSON.parse(localStorage.getItem("apmm.appearance-cache") || "null"),
      );
    } catch {
      return defaultAppearance;
    }
  });
  const [preview, setPreview] = useState<Preview>(null);
  const [error, setError] = useState("");
  const latest = useRef(appearance);
  const pendingSave = useRef(Promise.resolve());
  latest.current = appearance;
  const [dark, setDark] = useState(
    () => matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const migrated = useRef(false);
  const legacy = useRef({
    mode: localStorage.getItem("apmm.theme"),
    reduceMotion: localStorage.getItem("apmm.reduce-motion") === "true",
    catAnimation: localStorage.getItem("apmm.cat-mode") === "true",
  });
  const save = (next: AppearanceSettings) => {
    // 只合并本次编辑的字段，串行保存，避免连续切换两个选项时丢失前一次修改。
    const changes = Object.fromEntries(
      Object.entries(next).filter(
        ([key, value]) =>
          JSON.stringify(value) !==
          JSON.stringify(appearance[key as keyof AppearanceSettings]),
      ),
    );
    const operation = pendingSave.current
      .catch(() => {})
      .then(async () => {
        try {
          const result = await appService.settings({
            appearance: { ...latest.current, ...changes },
          });
          const saved = readAppearance(result.appearance);
          latest.current = saved;
          client.setQueryData<WorkspaceSnapshot>(["workspace"], (old) =>
            old
              ? { ...old, settings: { ...old.settings, appearance: saved } }
              : old,
          );
          setAppearance(saved);
          setError("");
        } catch (e) {
          setError(String(e));
          throw e;
        }
      });
    pendingSave.current = operation;
    return operation;
  };
  useEffect(() => {
    const settings = workspace.data?.settings;
    if (!settings) return;
    if (settings.appearance) {
      setAppearance(readAppearance(settings.appearance));
      return;
    }
    if (migrated.current) return;
    migrated.current = true;
    const mode = legacy.current.mode || "system";
    void save({
      ...defaultAppearance,
      ...legacy.current,
      mode: ["light", "dark", "system"].includes(mode) ? mode : "system",
    }).catch((e) => {
      migrated.current = false;
      setError(String(e));
    });
  }, [workspace.data?.settings.appearance, !!workspace.data]);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const listener = () => setDark(media.matches);
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, []);
  useLayoutEffect(() => {
    const mode =
      preview?.mode ||
      (appearance.mode === "system"
        ? dark
          ? "dark"
          : "light"
        : (appearance.mode as PaletteMode));
    applyPalette((preview?.scheme || selectedScheme(appearance))[mode], mode);
    localStorage.setItem("apmm.appearance-cache", JSON.stringify(appearance));
    localStorage.setItem("apmm.theme", appearance.mode);
    localStorage.setItem("apmm.reduce-motion", String(appearance.reduceMotion));
    localStorage.setItem("apmm.cat-mode", String(appearance.catAnimation));
    document.documentElement.dataset.reduceMotion = String(
      appearance.reduceMotion,
    );
  }, [appearance, preview, dark]);
  useEffect(() => {
    const visibility = () => {
      document.documentElement.dataset.hidden = String(document.hidden);
    };
    visibility();
    document.addEventListener("visibilitychange", visibility);
    return () => document.removeEventListener("visibilitychange", visibility);
  }, []);
  return (
    <Context.Provider
      value={{
        appearance,
        save,
        preview: setPreview,
        mode: appearance.mode,
        setMode: (mode) => {
          void save({ ...appearance, mode }).catch((e) => setError(String(e)));
        },
        error,
      }}
    >
      {children}
    </Context.Provider>
  );
}
