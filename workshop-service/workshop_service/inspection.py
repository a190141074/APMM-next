"""有界 ZIP/PNG 检查，只把已校验条目写入私有目录，不运行投稿内容。"""
from __future__ import annotations
import hashlib
import io
import json
import re
import warnings
import zipfile
from pathlib import Path
from PIL import Image
from .models import Submission, MAX_EXPANDED, MAX_IMAGE, MAX_ARCHIVE, MAX_UPLOAD

Image.MAX_IMAGE_PIXELS = 64 * 1024 * 1024

def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def png(data: bytes):
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("资源不是 PNG")
    with warnings.catch_warnings():
        warnings.simplefilter("error", Image.DecompressionBombWarning)
        with Image.open(io.BytesIO(data)) as image:
            if image.format != "PNG" or max(image.size) > 16384 or image.width * image.height * 4 > MAX_EXPANDED or getattr(image, "n_frames", 1) != 1:
                raise ValueError("PNG 解码超限或为动画")
            image.verify()
        with Image.open(io.BytesIO(data)) as image:
            image.load()

def unique_json(data: bytes):
    def unique(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError("重复 JSON 字段")
            value[key] = item
        return value
    return json.loads(data, object_pairs_hook=unique)

def classified(canonical: str, allowed: set[str]) -> bool:
    # 审核服务只接受随受信构建生成的完整官方名称；新映射需先更新服务目录。
    return canonical in allowed

def inspect(archive: Path, output: Path, allowed: set[str]) -> Submission:
    if archive.stat().st_size > MAX_UPLOAD:
        raise ValueError("投稿包超过 80 MiB")
    with zipfile.ZipFile(archive) as source:
        names = set()
        if not 1 <= len(source.infolist()) <= 520:
            raise ValueError("条目数量超限")
        for entry in source.infolist():
            if (entry.filename in {".", ".."} or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", entry.filename)
                or entry.is_dir() or (entry.external_attr >> 16) & 0o170000 == 0o120000
                or entry.filename.casefold() in names or entry.flag_bits & 1):
                raise ValueError("ZIP 路径、重复条目或加密方式无效")
            names.add(entry.filename.casefold())
        info = source.getinfo("submission.json")
        if info.file_size > 64 * 1024:
            raise ValueError("投稿资料超限")
        submission = Submission.model_validate(unique_json(source.read(info)))
        if submission.metadataOnly != (len(submission.files) == 0):
            raise ValueError("资料更新和内容更新的文件范围不匹配")
        entries = submission.files + submission.images
        if len({entry.name.casefold() for entry in entries}) != len(entries) or names != {"submission.json", *(e.name.casefold() for e in entries)}:
            raise ValueError("ZIP 与声明清单不一致")
        if sum(e.bytes for e in submission.files) > MAX_EXPANDED or sum(e.bytes for e in submission.images) > 7 * MAX_IMAGE:
            raise ValueError("展开大小超限")
        output.mkdir(parents=True, exist_ok=True)
        for entry in entries:
            is_content = entry in submission.files
            if is_content and not classified(entry.canonical, allowed):
                raise ValueError("资源尚未完成受支持的分类")
            if not is_content and (entry.canonical or entry.bytes > MAX_IMAGE):
                raise ValueError("预览图片无效")
            if source.getinfo(entry.name).file_size != entry.bytes:
                raise ValueError("文件大小不符")
            with source.open(entry.name) as stream:
                data = stream.read(entry.bytes + 1)
            if len(data) != entry.bytes or hashlib.sha256(data).hexdigest() != entry.sha256:
                raise ValueError("文件摘要不符")
            png(data)
            (output / entry.name).write_bytes(data)
        if submission.files:
            with zipfile.ZipFile(output / "content.zip", "w", compression=zipfile.ZIP_DEFLATED) as content:
                for entry in submission.files:
                    content.writestr(zipfile.ZipInfo(entry.name), (output / entry.name).read_bytes(), compress_type=zipfile.ZIP_DEFLATED)
            if (output / "content.zip").stat().st_size > MAX_ARCHIVE:
                raise ValueError("内容 ZIP 超过 64 MiB")
        return submission
