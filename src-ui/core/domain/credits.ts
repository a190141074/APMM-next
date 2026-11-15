// 从分发声明提取依赖归属，许可证正文原样保留，不把第三方许可当成本项目许可。
export function parseCredits(text: string) {
  return text
    .split(/(?=^## .+\r?\n\r?\n声明许可：)/m)
    .slice(1)
    .map((entry) => {
      const title = entry.slice(3, entry.indexOf("\n")).trim();
      const license = entry.match(/^声明许可：(.+)$/m)?.[1] || "见许可证原文";
      return { title, license, text: entry.replace(/\n\n---\n\n$/, "") };
    });
}

// 主页面只列用户可感知的主要运行依赖，完整清单在声明窗口展示。
export const creditGroups = [
  {
    title: "界面",
    items: [
      ["React", "界面框架", "MIT", "https://react.dev/"],
      ["Tailwind CSS", "样式", "MIT", "https://tailwindcss.com/"],
      ["Radix UI", "交互组件", "MIT", "https://www.radix-ui.com/"],
      ["TanStack Query", "状态查询", "MIT", "https://tanstack.com/query"],
      ["Lucide", "图标", "ISC", "https://lucide.dev/"],
    ],
  },
  {
    title: "桌面",
    items: [
      ["Tauri", "桌面框架", "MIT / Apache-2.0", "https://tauri.app/"],
      ["Tokio", "异步任务", "MIT", "https://tokio.rs/"],
    ],
  },
] as const;
