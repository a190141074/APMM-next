// 界面、favicon 和 Windows 图标从同一 SVG 渲染，不再使用旧版 ICO。
import { chromium } from "playwright";
import { readFileSync, existsSync } from "node:fs";
import path from "node:path";
process.env.TEMP = "B:/APMM-next-work/temp";
process.env.TMP = process.env.TEMP;
const browserPath = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
const browser = await chromium.launch({
  executablePath: browserPath,
  headless: true,
});
try {
  const page = await browser.newPage({
    viewport: { width: 256, height: 256 },
    deviceScaleFactor: 2,
  });
  await page.setContent(
    `<style>body{margin:0;background:transparent}</style>${readFileSync("public/logo.svg", "utf8")}`,
  );
  await page.screenshot({
    path: path.resolve("src-tauri/icons/icon.png"),
    omitBackground: true,
  });
} finally {
  await browser.close();
}
