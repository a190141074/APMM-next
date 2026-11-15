// 发布前只检查 Git 候选源码，输出风险路径，不输出匹配到的凭据内容。
import { execFileSync } from "node:child_process";
import { readFileSync, statSync } from "node:fs";
import path from "node:path";
const root = path.resolve(import.meta.dirname, "..");
const files = [...new Set(execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], { cwd: root, encoding: "utf8" }).split("\0").filter(Boolean))];
// 新增忽略规则不会移除已经跟踪的文件，因此必须单独阻止这类遗漏。
const ignoredTracked = new Set(execFileSync("git", ["ls-files", "--cached", "--ignored", "--exclude-standard", "-z"], { cwd: root, encoding: "utf8" }).split("\0").filter(Boolean));
const issues = [];
let bytes = 0;
for (const file of files) {
  if (ignoredTracked.has(file)) issues.push({ file, reason: "ignored-but-tracked" });
  const data = readFileSync(path.join(root, file)); bytes += data.length;
  if (statSync(path.join(root, file)).size > 50 * 1024 * 1024) issues.push({ file, reason: "oversized" });
  if (/(^|\/)(data|credentials|signing|previews|node_modules|target-msvc)\//i.test(file)) issues.push({ file, reason: "private-or-generated-directory" });
  if (data.includes(0)) continue;
  const content = data.toString("utf8");
  if (/-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|\bgh[pousr]_[A-Za-z0-9]{30,}\b|github_pat_[A-Za-z0-9_]{30,}|\bAKIA[A-Z0-9]{16}\b/.test(content)) issues.push({ file, reason: "credential-pattern" });
  if (/\b(?:Bearer|Basic)\s+[A-Za-z0-9._~+/-]{32,}={0,2}/i.test(content)) issues.push({ file, reason: "embedded-authorization" });
  if (/https?:\/\/[^/\s:@]+:[^/\s@]{3,}@/i.test(content)) issues.push({ file, reason: "credential-in-url" });
  if (/[A-Za-z]:[\\/]+Users[\\/]+(?!Public(?:[\\/]|\b)|Default(?:[\\/]|\b))[^\s\\/"'<>]+/i.test(content)) issues.push({ file, reason: "personal-profile-path" });
}
console.log(JSON.stringify({ files: files.length, bytes, issues }, null, 2));
if (issues.length) process.exitCode = 1;
