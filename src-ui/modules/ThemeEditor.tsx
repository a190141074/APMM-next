// 编辑器隔离未保存草稿；预览只改变显示，离开时明确保存或丢弃。
import { useEffect, useState } from "react";
import { Modal } from "../shared/Ui";
import { SegmentedTabs } from "../shared/SegmentedTabs";
import { useAppearance } from "../hooks/useAppearance";
import {
  colorRoles,
  contrast,
  defaultScheme,
  validateScheme,
  type PaletteMode,
  type ThemePalette,
  type ThemeScheme,
} from "../core/domain/appearance";
import { ColorControl } from "./ColorControl";

export function ThemeEditor({
  initial,
  onClose,
}: {
  initial: ThemeScheme;
  onClose: () => void;
}) {
  const { appearance, save, preview } = useAppearance();
  const [draft, setDraft] = useState(() => structuredClone(initial));
  const [mode, setMode] = useState<PaletteMode>(() =>
    document.documentElement.dataset.theme === "dark" ? "dark" : "light",
  );
  const [role, setRole] = useState<keyof ThemePalette>("accent");
  const [closing, setClosing] = useState(false),
    [saving, setSaving] = useState(false),
    [picking, setPicking] = useState(false),
    [error, setError] = useState("");
  const isNew = !appearance.schemes.some((s) => s.id === initial.id);
  const dirty = isNew || JSON.stringify(initial) !== JSON.stringify(draft);
  useEffect(() => {
    preview({ scheme: draft, mode });
  }, [draft, mode, preview]);
  useEffect(() => () => preview(null), [preview]);
  const updateColor = (value: string) =>
    setDraft((s) => ({ ...s, [mode]: { ...s[mode], [role]: value } }));
  const finish = async () => {
    setSaving(true);
    setError("");
    try {
      const scheme = validateScheme(draft);
      await save({
        ...appearance,
        activeSchemeId: scheme.id,
        schemes: [
          ...appearance.schemes.filter((s) => s.id !== scheme.id),
          scheme,
        ],
      });
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };
  const palette = draft[mode];
  const warnings = [
    ["主文字 / 面板", palette.text, palette.surface, "text"],
    ["主文字 / 页面", palette.text, palette.canvas, "text"],
    ["辅助文字 / 面板", palette.muted, palette.surface, "muted"],
    ["按钮文字 / 强调色", palette.onAccent, palette.accent, "onAccent"],
  ].filter(
    ([, foreground, background]) => contrast(foreground, background) < 4.5,
  );
  return (
    <Modal
      open
      wide
      title="编辑配色"
      className={`theme-editor ${picking ? "is-picking" : ""}`}
      onClose={() => {
        if (!saving && !picking) dirty ? setClosing(true) : onClose();
      }}
    >
      <div className="theme-editor-tools safe-controls">
        <label>
          方案名称
          <input
            aria-label="方案名称"
            value={draft.name}
            maxLength={80}
            onChange={(e) => setDraft((s) => ({ ...s, name: e.target.value }))}
          />
        </label>
        <SegmentedTabs
          label="编辑亮暗配色"
          value={mode}
          options={[
            { value: "light", label: "亮色方案" },
            { value: "dark", label: "暗色方案" },
          ]}
          onChange={(v) => setMode(v as PaletteMode)}
        />
      </div>
      <div className="theme-editor-body">
        <div className="theme-roles safe-controls" aria-label="颜色部位">
          {colorRoles.map(({ key, label, group }, index) => (
            <div key={key}>
              {(index === 0 || colorRoles[index - 1].group !== group) && (
                <small className="role-group">{group}</small>
              )}
              <button
                className={role === key ? "selected" : ""}
                aria-pressed={role === key}
                onClick={() => setRole(key)}
              >
                <span
                  className="color-chip"
                  style={{ background: palette[key] }}
                />
                {label}
              </button>
            </div>
          ))}
        </div>
        <div className="theme-controls safe-controls">
          <ColorControl
            key={role + mode}
            label={colorRoles.find((r) => r.key === role)!.label}
            value={palette[role]}
            onChange={updateColor}
            onPicking={setPicking}
          />
          <button
            className="text-button"
            onClick={() => updateColor(defaultScheme[mode][role])}
          >
            重置此颜色
          </button>
        </div>
        <div className="theme-live-preview">
          <small>实时预览</small>
          <div className="theme-sample">
            <h3>你的工作区</h3>
            <p className="muted">原图、Mod 与灵感，各就其位。</p>
            <div className="theme-sample-image">Aa</div>
            <input readOnly value="搜索资源" aria-label="配色示例输入" />
            <div className="action-row">
              <button className="button primary" type="button">
                应用
              </button>
              <button className="button" type="button">
                查看
              </button>
            </div>
            <div className="action-row">
              <span className="badge success">成功</span>
              <span className="badge warning">警告</span>
              <span className="badge danger">错误</span>
              <span className="badge info">信息</span>
            </div>
          </div>
          <p className="muted">整个工作区同步试色，保存后才会记住。</p>
        </div>
      </div>
      <footer className="theme-editor-footer safe-controls">
        {warnings.length > 0 && (
          <details className="contrast-warning">
            <summary>有 {warnings.length} 组文字对比度偏低</summary>
            {warnings.map(([label, foreground, background, key]) => (
              <div key={label}>
                {label} · {contrast(foreground, background).toFixed(2)}:1{" "}
                <button
                  className="text-button"
                  onClick={() =>
                    setDraft((s) => ({
                      ...s,
                      [mode]: {
                        ...s[mode],
                        [key]:
                          contrast("#000000", background) >
                          contrast("#ffffff", background)
                            ? "#000000"
                            : "#ffffff",
                      },
                    }))
                  }
                >
                  改善文字对比
                </button>
              </div>
            ))}
          </details>
        )}
        {error && <p role="alert">{error}</p>}
        <div className="action-row">
          <button
            className="button"
            disabled={saving || picking}
            onClick={() =>
              setDraft((s) => ({
                ...s,
                [mode]: structuredClone(defaultScheme[mode]),
              }))
            }
          >
            重置当前亮暗方案
          </button>
          <button
            className="button"
            disabled={saving || picking}
            onClick={() =>
              setDraft((s) => ({
                ...s,
                light: structuredClone(defaultScheme.light),
                dark: structuredClone(defaultScheme.dark),
              }))
            }
          >
            重置整套配色
          </button>
          <span className="grow" />
          <button
            className="button"
            disabled={saving || picking}
            onClick={() => (dirty ? setClosing(true) : onClose())}
          >
            取消
          </button>
          <button
            className="button primary"
            disabled={saving || picking || !draft.name.trim()}
            onClick={() => void finish()}
          >
            {saving ? "保存中…" : "保存配色"}
          </button>
        </div>
      </footer>
      {closing && (
        <Modal
          open
          title="配色尚未保存"
          className="safe-controls"
          onClose={() => setClosing(false)}
        >
          <div className="wizard-body">
            <p>保存这次修改，或丢弃草稿并恢复已保存配色。</p>
            <div className="action-row">
              <button className="button" onClick={() => setClosing(false)}>
                继续编辑
              </button>
              <button className="button" onClick={onClose}>
                丢弃修改
              </button>
              <button
                className="button primary"
                disabled={saving}
                onClick={() => void finish()}
              >
                保存并关闭
              </button>
            </div>
            {error && <p role="alert">{error}</p>}
          </div>
        </Modal>
      )}
    </Modal>
  );
}
