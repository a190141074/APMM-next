// 总览展示工作区状态、入口与社交媒体，导航操作由应用外壳负责。
import { ArrowRight, Blocks, Puzzle, Sparkles, Users } from "lucide-react";
import { Badge } from "../shared/Ui";
import { SocialMedia } from "./SocialMedia";
import { OverviewVersions } from "./OverviewVersions";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
import type { Route } from "../core/domain/resources";
export function OverviewPage({
  snapshot,
  running,
  native,
  navigate,
}: {
  snapshot: WorkspaceSnapshot;
  running: number;
  native: boolean;
  navigate: (route: Route) => void;
}) {
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">YOUR CREATIVE SPACE</div>
          <h1>让每一份灵感，各就其位。</h1>
          <p>浏览原图、管理替换，定制你的 Astral Party。</p>
        </div>
        <Badge tone="success">{native ? "桌面工作区" : "交互预览"}</Badge>
      </div>
      <div className="overview-social-row">
        <div className="overview-hero">
          <div>
            <span className="eyebrow">ASTRAL PARTY</span>
            <h2>先安装 APX，再应用 Mod。</h2>
            <p>Mod 需要 APX 支持。请选择游戏渠道，确认适配版本后安装。</p>
            <button className="button primary" onClick={() => navigate("apx")}>
              安装 APX 插件 <ArrowRight size={17} />
            </button>
          </div>
          <div className="hero-orbit">
            <Sparkles size={90} />
            <span>✦</span>
            <b>AP</b>
          </div>
        </div>
        <div className="overview-side">
          <OverviewVersions
            version={snapshot.version}
            platform={snapshot.settings.pluginPlatform || "CN_STEAM"}
            navigate={navigate}
          />
          <SocialMedia />
        </div>
      </div>
      <div className="stats-grid">
        {[
          ["资源槽位", snapshot.resources.length],
          [
            "已收藏 Mod",
            snapshot.resources.reduce((n, r) => n + r.files.length, 0),
          ],
          ["正在应用", snapshot.resources.filter((r) => r.activeHash).length],
          ["进行中任务", running],
        ].map(([label, value]) => (
          <div className="stat" key={label}>
            <small>{label}</small>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
      <div className="section-heading">
        <h2>从这里开始</h2>
        <span>先准备插件，再管理 Mod</span>
      </div>
      <div className="quick-grid">
        {[
          ["apx", "1. 安装 APX", "Mod 所需的客户端补丁", Blocks],
          ["heroes", "2. 管理 Mod", "浏览、导入并应用 Mod", Users],
          [
            "auxiliary",
            "3. 提取原图",
            "按需安装资产工具，保留完整画质",
            Puzzle,
          ],
        ].map(([key, title, description, Icon]: any) => (
          <button
            className="quick-card"
            key={key}
            onClick={() => navigate(key)}
          >
            <Icon size={24} />
            <h3>{title}</h3>
            <p>{description}</p>
            <ArrowRight size={17} />
          </button>
        ))}
      </div>
    </>
  );
}
