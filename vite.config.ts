import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwind from "@tailwindcss/vite";
import { localPreview } from "./tools/local-preview";
export default defineConfig({
  plugins: [react(), tailwind(), localPreview()],
  server: { port: 1420, strictPort: true },
  build: { outDir: "src-release", emptyOutDir: false },
  clearScreen: false,
});

// 开发预览使用公共目录中的统一 Logo。
