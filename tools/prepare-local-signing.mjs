// 为本地验收生成独立签名身份，私钥只写入源码目录以外的用户配置目录。
import { generateKeyPairSync } from "node:crypto";
import { mkdirSync, existsSync, writeFileSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
const keyDir = path.join(
  process.env.APMM_WORK_DIR || "B:/APMM-next-work",
  "signing",
);
mkdirSync(keyDir, { recursive: true });
const keyFile = path.join(keyDir, "asset-tool-local.pem");
let publicKey;
if (!existsSync(keyFile)) {
  const keys = generateKeyPairSync("ed25519");
  writeFileSync(
    keyFile,
    keys.privateKey.export({ format: "pem", type: "pkcs8" }),
    { mode: 0o600, flag: "wx" },
  );
  publicKey = keys.publicKey;
} else {
  const { createPublicKey } = await import("node:crypto");
  publicKey = createPublicKey(readFileSync(keyFile));
}
writeFileSync(
  "config/auxiliary.json",
  JSON.stringify(
    {
      schema: 1,
      publicKey: publicKey
        .export({ format: "der", type: "spki" })
        .subarray(-32)
        .toString("hex"),
      baseUrl:
        "https://cnb.cool/astral-weird-egg/astral-party-mod-manager-assets/-/git/raw/main/auxiliary/asset-tool",
      channel: "local-acceptance",
    },
    null,
    2,
  ) + "\n",
);
console.log("本地验收公钥已生成。私钥保留在用户配置目录，不进入源码或发行包。");
