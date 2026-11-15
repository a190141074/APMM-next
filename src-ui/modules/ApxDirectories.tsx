// 目录修改绑定打开时的平台；智能匹配先展示候选，不自动选择多份安装中的首项。
import { useState } from "react";
import { useActions } from "../hooks/useApp";
import { IPC } from "../core/ipc/commands";
import type { PluginDirectories } from "../core/ipc/generated/PluginDirectories";
import { Modal } from "../shared/Ui";
const fields = [
  ["locallow", "LocalLow 目录", "locallowDir"],
  ["data", "游戏 Data 目录", "gameDataDir"],
  ["root", "游戏根目录", "gameRootDir"],
] as const;
export function ApxDirectories({
  platform,
  platformLabel,
  data,
  waiting,
  onRefresh,
}: {
  platform: string;
  platformLabel: string;
  data: Record<string, any> | undefined;
  waiting: boolean;
  onRefresh: () => Promise<unknown>;
}) {
  const { api, act, busy, notice } = useActions();
  const [match, setMatch] = useState<{
    platform: string;
    label: string;
    which: string;
    paths: string[];
  } | null>(null);
  const disabled = waiting || busy || platform === "CN_ANDROID";
  const save = async (which: string, path: string) => {
    await api.command<PluginDirectories>(IPC.plugin_set_directory, {
      platform,
      which,
      path,
    });
    await onRefresh();
  };
  return (
    <>
      <details className="settings-card" open>
        <summary>目录设置与定位 · {platformLabel}</summary>
        <p className="muted">
          各平台单独保存目录。此处修改仅用于 {platformLabel} 的补丁。
        </p>
        {fields.map(([which, label, field]) => (
          <div className="directory-row" key={which}>
            <div>
              <strong>{label}</strong>
              <code>{data?.[field] || "尚未定位"}</code>
            </div>
            <button
              className="button"
              disabled={disabled}
              onClick={() =>
                void act(async () => {
                  const paths = await api.command<string[]>(
                    IPC.plugin_match_directory,
                    { platform, which },
                  );
                  setMatch({ platform, label, which, paths });
                }, "已完成目录匹配")
              }
            >
              智能匹配
            </button>
            <button
              className="button"
              disabled={disabled}
              onClick={() =>
                void act(async () => {
                  const dir = await api.chooseDirectory();
                  if (dir) await save(which, dir);
                }, "当前平台的目录已保存")
              }
            >
              更改
            </button>
            <button
              className="button"
              disabled={disabled || !data?.[field]}
              onClick={() =>
                void act(() =>
                  api.command(IPC.plugin_open_dir, { platform, which }),
                )
              }
            >
              打开
            </button>
          </div>
        ))}
        {notice && (
          <div className="inline-notice" role="status">
            {notice}
          </div>
        )}
      </details>
      {match && (
        <Modal
          open
          title={`智能匹配 · ${match.label}`}
          onClose={() => setMatch(null)}
        >
          <div className="wizard-body">
            <p>
              为 {platformLabel} 选择目录。选择后仅更新这个平台的{match.label}。
            </p>
            {!match.paths.length && (
              <p>
                未找到可确认的目录，请使用“更改”手动选择。TapTap、哔哩哔哩与
                WeGame 使用同名 Data 目录，需要先分别指定游戏根目录。
              </p>
            )}
            {match.paths.map((path) => (
              <div className="directory-candidate" key={path}>
                <code>{path}</code>
                <button
                  className="button primary"
                  disabled={disabled || match.platform !== platform}
                  onClick={() =>
                    void act(async () => {
                      await save(match.which, path);
                      setMatch(null);
                    }, "当前平台的目录已保存")
                  }
                >
                  使用此目录
                </button>
              </div>
            ))}
          </div>
        </Modal>
      )}
    </>
  );
}
