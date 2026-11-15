// 普通外观偏好即时持久化，配色编辑器另行维护显式保存的草稿。
import { useAppearance } from "../hooks/useAppearance";
import { ThemedSelect } from "../shared/ThemedSelect";
export function AppearanceSettingsPanel() {
  const { appearance, mode, setMode, save } = useAppearance();
  return (
    <>
      <section className="settings-card">
        <h2>界面</h2>
        <div className="settings-row">
          <span>
            主题模式<small>亮暗模式使用当前配色方案中的对应颜色。</small>
          </span>
          <ThemedSelect
            label="主题"
            value={mode}
            onValueChange={setMode}
            options={[
              { value: "system", label: "跟随系统" },
              { value: "light", label: "浅色" },
              { value: "dark", label: "深色" },
            ]}
          />
        </div>
      </section>
      <section className="settings-card">
        <h2>动效</h2>
        <label className="settings-row">
          <span>
            减少动画<small>停用装饰动效，保留操作反馈。</small>
          </span>
          <input
            type="checkbox"
            checked={appearance.reduceMotion}
            onChange={(e) =>
              void save({
                ...appearance,
                reduceMotion: e.target.checked,
              }).catch(() => {})
            }
          />
        </label>
        <label className="settings-row">
          <span>
            加载时显示吸猫动画<small>减少动画或系统要求静态显示时暂停。</small>
          </span>
          <input
            type="checkbox"
            checked={appearance.catAnimation}
            onChange={(e) =>
              void save({
                ...appearance,
                catAnimation: e.target.checked,
              }).catch(() => {})
            }
          />
        </label>
      </section>
    </>
  );
}
