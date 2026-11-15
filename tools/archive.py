"""生成仅含指定目录文件的 ZIP，固定时间戳使清单审计易于复现。"""
import sys
import zipfile
from pathlib import Path

root, output = map(Path, sys.argv[1:3])
with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for path in sorted(root.rglob('*')):
        if path.is_file():
            info = zipfile.ZipInfo(path.relative_to(root).as_posix(), (2026, 11, 14, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, path.read_bytes())
print(output)
