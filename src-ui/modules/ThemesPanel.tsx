// 方案列表只提交完整配色，导入文件先校验，再以新标识进入编辑器。
import { useRef, useState } from "react";
import { Copy, Download, Plus, Upload } from "lucide-react";
import { useAppearance } from "../hooks/useAppearance";
import {
  defaultScheme,
  importScheme,
  type ThemeScheme,
} from "../core/domain/appearance";
import { saveSchemeFile } from "../core/services/appearance";
import { Modal } from "../shared/Ui";
import { ThemeEditor } from "./ThemeEditor";

export function ThemesPanel() {
  const { appearance, save } = useAppearance();
  const [editor, setEditor] = useState<ThemeScheme | null>(null),
    [deleting, setDeleting] = useState<ThemeScheme | null>(null);
  const [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const full = appearance.schemes.length >= 64;
  const act = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    setError("");
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const copy = (s: ThemeScheme, name = `${s.name} 副本`) =>
    setEditor({
      ...structuredClone(s),
      id: crypto.randomUUID(),
      name: name.slice(0, 80),
    });
  return (
    <section className="theme-collection">
      <div className="section-heading">
        <div>
          <h2>配色方案</h2>
          <p className="muted">每套方案独立保存亮色与暗色配色。</p>
        </div>
        <div className="action-row">
          <button
            className="button"
            disabled={full || busy}
            onClick={() => input.current?.click()}
          >
            <Upload size={14} />
            导入
          </button>
          <button
            className="button primary"
            disabled={full || busy}
            onClick={() => copy(defaultScheme, "我的配色")}
          >
            <Plus size={14} />
            新建方案
          </button>
        </div>
      </div>
      <input
        ref={input}
        type="file"
        accept=".json,application/json"
        className="sr-only"
        aria-label="导入配色文件"
        onChange={(e) => {
          const file = e.target.files?.[0];
          e.target.value = "";
          if (file)
            void act(async () => {
              if (file.size > 65536) throw Error("配色文件超过 64 KiB");
              setEditor(importScheme(await file.text()));
            });
        }}
      />
      <div className="scheme-grid">
        {[defaultScheme, ...appearance.schemes].map((s) => (
          <article
            className={`scheme-card ${s.id === appearance.activeSchemeId ? "selected" : ""}`}
            key={s.id}
          >
            <button
              className="scheme-select"
              aria-pressed={s.id === appearance.activeSchemeId}
              disabled={busy}
              onClick={() =>
                void act(() => save({ ...appearance, activeSchemeId: s.id }))
              }
            >
              <div className="scheme-swatches">
                {(["light", "dark"] as const).map((mode) => (
                  <div
                    style={{ background: s[mode].canvas, color: s[mode].text }}
                    key={mode}
                  >
                    <span style={{ background: s[mode].surface }}>Aa</span>
                    <i style={{ background: s[mode].accent }} />
                    <i style={{ background: s[mode].success }} />
                  </div>
                ))}
              </div>
              <strong>{s.name}</strong>
              <small>
                {s.id === appearance.activeSchemeId ? "使用中" : "选择此方案"}
              </small>
            </button>
            <div className="scheme-actions">
              <button
                className="text-button"
                disabled={full || busy}
                onClick={() => copy(s)}
                title="复制方案"
              >
                <Copy size={13} />
                复制
              </button>
              <button
                className="text-button"
                disabled={busy}
                onClick={() => void act(() => saveSchemeFile(s))}
              >
                <Download size={13} />
                导出
              </button>
              {s.id !== "default" && (
                <>
                  <button
                    className="text-button"
                    disabled={busy}
                    onClick={() => setEditor(s)}
                  >
                    编辑 / 重命名
                  </button>
                  <button
                    className="text-button danger"
                    disabled={busy}
                    onClick={() => setDeleting(s)}
                  >
                    删除
                  </button>
                </>
              )}
            </div>
          </article>
        ))}
      </div>
      {error && <p role="alert">{error}</p>}
      {editor && (
        <ThemeEditor initial={editor} onClose={() => setEditor(null)} />
      )}
      {deleting && (
        <Modal open title="删除配色方案" onClose={() => setDeleting(null)}>
          <div className="wizard-body">
            <p>删除「{deleting.name}」？正在使用的方案会切回 APMM 默认配色。</p>
            <button
              className="button"
              disabled={busy}
              onClick={() =>
                void act(async () => {
                  await save({
                    ...appearance,
                    activeSchemeId:
                      appearance.activeSchemeId === deleting.id
                        ? "default"
                        : appearance.activeSchemeId,
                    schemes: appearance.schemes.filter(
                      (s) => s.id !== deleting.id,
                    ),
                  });
                  setDeleting(null);
                })
              }
            >
              确认删除
            </button>
          </div>
        </Modal>
      )}
    </section>
  );
}
