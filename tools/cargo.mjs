import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import os from "node:os";
const root = path.resolve(import.meta.dirname, "..");
const windows = process.platform === "win32";
const rust =
  process.env.APMM_RUST_BIN ||
  (windows &&
  existsSync(
    "B:/Rust/rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe",
  )
    ? "B:/Rust/rustup/toolchains/stable-x86_64-pc-windows-msvc/bin"
    : "");
const vs =
  process.env.APMM_VSDEV || "B:\\VS\\BuildTools\\Common7\\Tools\\VsDevCmd.bat";
const mode = process.argv[2] || "check";
const args =
  mode === "check"
    ? ["check", "--manifest-path", "src-tauri/Cargo.toml", "--all-targets"]
    : mode === "test"
      ? [
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          ...process.argv.slice(3),
        ]
      : mode === "build"
        ? ["build", "--release", "--manifest-path", "src-tauri/Cargo.toml"]
        : mode === "metadata"
          ? [
              "metadata",
              "--manifest-path",
              "src-tauri/Cargo.toml",
              "--format-version",
              "1",
              "--locked",
              "--offline",
              "--filter-platform",
              "x86_64-pc-windows-msvc",
            ]
          : mode === "fmt"
            ? [
                "fmt",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                ...process.argv.slice(3),
              ]
            : [
                "run",
                "--bin",
                "astral-mod-manager",
                "--manifest-path",
                "src-tauri/Cargo.toml",
              ];
const work =
  process.env.APMM_WORK_DIR ||
  (windows && existsSync("B:/")
    ? "B:/APMM-next-work"
    : path.join(os.tmpdir(), "apmm-next-work"));
const temp = path.join(work, "temp");
mkdirSync(temp, { recursive: true });
const env = {
  ...process.env,
  APMM_WORK_DIR: work,
  TEMP: temp,
  TMP: temp,
  PATH: rust ? `${rust}${path.delimiter}${process.env.PATH}` : process.env.PATH,
  CARGO_TARGET_DIR:
    process.env.CARGO_TARGET_DIR ||
    path.join(root, "src-tauri", windows ? "target-msvc" : "target"),
  CARGO_BUILD_JOBS: "2",
  RUST_TEST_THREADS: "2",
};
if (rust && !existsSync(path.join(rust, windows ? "cargo.exe" : "cargo")))
  throw Error("请通过 APMM_RUST_BIN 指定可用的 MSVC Rust 工具链。");
const command = mode === "dev" ? process.execPath : "cargo";
const commandArgs =
  mode === "dev"
    ? [
        path.join(root, "node_modules/@tauri-apps/cli/tauri.js"),
        "dev",
        "--no-default-features",
      ]
    : args;
if (windows && !existsSync(vs))
  throw Error("请通过 APMM_VSDEV 指定 Visual Studio 的 VsDevCmd.bat。");
const result = windows
  ? spawnSync(
      "cmd.exe",
      [
        "/d",
        "/s",
        "/c",
        `call "${vs}" -arch=x64 -host_arch=x64 >nul && "${command}" ${commandArgs.map((x) => `"${x}"`).join(" ")}`,
      ],
      { cwd: root, env, stdio: "inherit", windowsVerbatimArguments: true },
    )
  : spawnSync(command, commandArgs, { cwd: root, env, stdio: "inherit" });
process.exit(result.status ?? 1);
