"""生成仅含指定目录文件的 ZIP，固定时间戳使清单审计易于复现。"""
import sys
import zipfile
from pathlib import Path

if len(sys.argv) not in (3, 4):
    raise SystemExit("用法：archive.py <payload> <output.zip> [top-level-folder]")

root, output = map(Path, sys.argv[1:3])
prefix = sys.argv[3] if len(sys.argv) == 4 else ""
if prefix and (Path(prefix).name != prefix or prefix in {".", ".."} or "/" in prefix or chr(92) in prefix or ":" in prefix):
    raise SystemExit("ZIP 顶层目录必须是单个文件夹名称")
prefix = f"{prefix}/" if prefix else ""
with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for path in sorted(root.rglob('*')):
        if path.is_file():
            name = prefix + path.relative_to(root).as_posix()
            info = zipfile.ZipInfo(name, (2026, 11, 14, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, path.read_bytes())
print(output)
