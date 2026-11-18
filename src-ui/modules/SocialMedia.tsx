// 默认展示全部社交入口；compact 保留原折叠版，便于链接增多后复用。
import { useRef, useState } from "react";
import { ExternalLink } from "lucide-react";
import * as Dialog from "@radix-ui/react-dialog";
import { useActions } from "../hooks/useApp";
export const socialLinks = [
  ["吉星派对", "https://space.bilibili.com/3546954038511957/"],
  ["BWIKI", "https://wiki.biligame.com/starengine/%E9%A6%96%E9%A1%B5"],
  ["英文 wiki", "https://astralparty.miraheze.org/wiki/Main_Page"],
  ["TAPTAP", "https://www.taptap.cn/app/524022"],
] as const;
export function SocialMedia({ compact = false }: { compact?: boolean }) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const { api, act, notice } = useActions();
  const links = (all: boolean) =>
    socialLinks.slice(0, all ? socialLinks.length : 3).map(([title, url]) => (
      <a
        className="social-link"
        key={title}
        href={url}
        onClick={(event) => {
          event.preventDefault();
          void act(() => api.openExternal(url), "已打开社交媒体页面");
        }}
      >
        <span>{title}</span>
        <ExternalLink size={13} />
      </a>
    ));
  return (
    <section className="settings-card social-card">
      <h2>
        社交媒体 <small>Social Media</small>
      </h2>
      <div className="social-links">{links(!compact)}</div>
      {compact && (
        <Dialog.Root open={open} onOpenChange={setOpen}>
          <Dialog.Trigger asChild>
            <button className="text-button" ref={trigger}>
              展开
            </button>
          </Dialog.Trigger>
          <Dialog.Portal>
            <Dialog.Overlay className="modal-overlay" />
            <Dialog.Content
              className="modal"
              onCloseAutoFocus={() => trigger.current?.focus()}
            >
              <header>
                <Dialog.Title>社交媒体（Social Media）</Dialog.Title>
                <Dialog.Close className="icon-button" aria-label="关闭">
                  ×
                </Dialog.Close>
              </header>
              <Dialog.Description className="sr-only">
                游戏社区、百科与资讯入口
              </Dialog.Description>
              <div className="wizard-body social-grid">{links(true)}</div>
            </Dialog.Content>
          </Dialog.Portal>
        </Dialog.Root>
      )}
      {notice && <small role="status">{notice}</small>}
    </section>
  );
}
