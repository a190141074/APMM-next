// 关于页按主要引用分组；完整第三方清单和许可证只在用户打开声明窗口时展示。
import { useMemo, useState } from "react";
import { ExternalLink } from "lucide-react";
import licenseText from "../../LICENSE.txt?raw";
import notices from "../../THIRD_PARTY_NOTICES.md?raw";
import { parseCredits, creditGroups } from "../core/domain/credits";
import { useActions } from "../hooks/useApp";
import { Badge, Modal } from "../shared/Ui";
export default function AboutPage({
  version,
  embedded = false,
}: {
  version: string;
  embedded?: boolean;
}) {
  const [filter, setFilter] = useState("");
  const [allLicenses, setAllLicenses] = useState(false);
  const [license, setLicense] = useState<{
    title: string;
    text: string;
  } | null>(null);
  const credits = useMemo(() => parseCredits(notices), []);
  const visible = credits.filter((credit) =>
    `${credit.title} ${credit.license}`
      .toLowerCase()
      .includes(filter.toLowerCase()),
  );
  const { api, act, notice } = useActions();
  const link = (title: string, url: string) => (
    <a
      href={url}
      target="_blank"
      rel="noreferrer"
      onClick={(e) => {
        e.preventDefault();
        void act(() => api.openExternal(url), "已打开项目页面");
      }}
    >
      {title}
      <ExternalLink size={12} />
    </a>
  );
  return (
    <div className="about-page">
      {!embedded && (
        <div className="page-heading">
          <h1>关于</h1>
        </div>
      )}
      <section className="settings-card about-application">
        <div className="about-app">
          <img src="/logo.svg" alt="APMM" />
          <div>
            <h2>Astral Party Mod Manager</h2>
            <small>管理、应用与分享游戏 Mod · GPL-3.0</small>
          </div>
          <Badge>v{version}</Badge>
        </div>
        <div className="about-links">
          {link("项目主页", "https://github.com/a190141074/APMM-next")}
          <button
            className="button"
            onClick={() => setLicense({ title: "许可说明", text: licenseText })}
          >
            许可说明
          </button>
          <button
            className="button"
            onClick={() => {
              setFilter("");
              setAllLicenses(true);
            }}
          >
            第三方声明
          </button>
        </div>
      </section>
      <div className="about-credit-groups">
        <section className="settings-card">
          <h2>鸣谢 · 参考项目</h2>
          <div className="credit-row">
            <div>
              <strong>NapCatQQ-Desktop</strong>
              <small>
                页面交互、紧凑字段与 APX
                状态卡布局、检查后直接安装的更新流程参考 · GPL-3.0
              </small>
            </div>
            {link("主页", "https://github.com/NapNeko/NapCatQQ-Desktop")}
            {link(
              "许可",
              "https://github.com/NapNeko/NapCatQQ-Desktop/blob/main/LICENSE",
            )}
          </div>
          <div className="credit-row">
            <div>
              <strong>旧版 Astral Party Mod Manager</strong>
              <small>角色紧凑卡、原图定位、预览下载提醒及模组输出行为参考</small>
            </div>
            {link(
              "发布与资源",
              "https://cnb.cool/astral-weird-egg/astral-party-mod-manager-assets",
            )}
          </div>
          <div className="credit-row">
            <div>
              <strong>JASM</strong>
              <small>模组管理体验参考</small>
            </div>
            {link("主页", "https://github.com/Jorixon/JASM")}
          </div>
          <div className="credit-row">
            <div>
              <strong>AstralParty.Toys</strong>
              <small>逐文件元数据缓存与启动扫描优化参考 · AGPL-3.0</small>
            </div>
            {link(
              "主页",
              "https://github.com/higashitaniyume/AstralParty.Toys",
            )}
            {link(
              "许可",
              "https://github.com/higashitaniyume/AstralParty.Toys/blob/master/LICENSE",
            )}
          </div>
          <div className="credit-row">
            <div>
              <strong>WorkshopUploaderMod</strong>
              <small>工坊投稿资料、预览图与提交流程参考 · AGPL-3.0</small>
            </div>
            {link(
              "主页",
              "https://github.com/BAKAOLC/STS2-WorkshopUploaderMod",
            )}
            {link(
              "许可",
              "https://github.com/BAKAOLC/STS2-WorkshopUploaderMod/blob/main/LICENSE",
            )}
          </div>
        </section>
        <section className="settings-card">
          <h2>鸣谢 · 工具</h2>
          <div className="credit-row">
            <div>
              <strong>APX</strong>
              <small>客户端补丁</small>
            </div>
            {link("发布页", "https://cnb.cool/erinilis/apx-plugin-releases")}
          </div>
          <div className="credit-row">
            <div>
              <strong>Codex</strong>
              <small>开发协作工具</small>
            </div>
            {link("主页", "https://openai.com/codex/")}
          </div>
        </section>
      </div>
      <div className="about-credit-groups">
        {creditGroups.map((group) => (
          <section className="settings-card" key={group.title}>
            <h2>鸣谢 · {group.title}</h2>
            {group.items.map(([name, role, license, url]) => (
              <div className="credit-row" key={name}>
                <div>
                  <strong>{name}</strong>
                  <small>
                    {role} · {license}
                  </small>
                </div>
                {link("主页", url)}
              </div>
            ))}
          </section>
        ))}
      </div>
      {notice && (
        <div className="inline-notice" role="status">
          {notice}
        </div>
      )}
      {(allLicenses || license) && (
        <Modal
          open
          wide
          title={license?.title || "第三方声明"}
          onClose={() => {
            setAllLicenses(false);
            setLicense(null);
          }}
        >
          <div className="wizard-body">
            {license ? (
              <>
                {allLicenses && (
                  <button className="button" onClick={() => setLicense(null)}>
                    返回清单
                  </button>
                )}
                <pre className="license-text">{license.text}</pre>
              </>
            ) : (
              <>
                <input
                  aria-label="查找第三方依赖"
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                  placeholder="React、Radix、MIT…"
                />
                <small>{visible.length} 项依赖与构建工具</small>
                <div className="credit-list">
                  {visible.map((credit) => (
                    <div className="credit-row" key={credit.title}>
                      <div>
                        <strong>{credit.title}</strong>
                        <small>{credit.license}</small>
                      </div>
                      <button
                        className="button"
                        onClick={() => setLicense(credit)}
                      >
                        许可证原文
                      </button>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        </Modal>
      )}
    </div>
  );
}
