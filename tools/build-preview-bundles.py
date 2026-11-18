"""从官方提取对应表生成有损 WebP 预览包，保留旧客户端的分类和文件集合。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import io
import json
from pathlib import Path
import zipfile
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ENCODING = {"format": "WEBP", "lossless": False, "quality": 50, "method": 4,
            "resized": False, "alpha": "lossless"}


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def build(args):
    """只接受摘要、尺寸均与 map 一致的原图；旧目录已移除的资源保留旧预览。"""
    output = args.output.resolve()
    if output == ROOT or ROOT in output.parents or output.exists():
        raise ValueError("输出必须是源码以外的新目录。")
    definitions = {section: row["key"] for row in read(ROOT / "config/preview-groups.json") for section in row["sections"]}
    catalog = read(ROOT / "config/manifest.json")
    old_manifest = read(args.legacy_manifest)
    old = {}
    old_groups = {}
    for bundle in old_manifest["bundles"]:
        file = args.legacy_bundles / bundle["bundleUrl"].rsplit("/", 1)[-1]
        raw = file.read_bytes()
        if len(raw) != bundle["bytes"] or digest(raw) != bundle["sha256"]:
            raise ValueError(f"旧包摘要不符：{file.name}")
        names = []
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            for name in archive.namelist():
                if name.endswith("/"):
                    continue
                path = Path(name)
                if len(path.parts) != 2 or path.parts[0] != "previews" or path.suffix != ".webp":
                    raise ValueError(f"旧包路径无效：{name}")
                names.append(path.name)
                data = archive.read(name)
                if path.name in old and old[path.name] != data:
                    raise ValueError(f"旧分类包存在同名不同内容：{path.name}")
                old[path.name] = data
        if len(names) != bundle["files"] or len(set(names)) != len(names):
            raise ValueError("旧包数量不符。")
        old_groups[bundle["key"]] = names
    maps = {}
    for file in sorted((args.assets / "maps").glob("*.json")):
        for row in read(file):
            if row["canonical"] in maps:
                raise ValueError("官方对应表存在重复完整名称。")
            maps[row["canonical"]] = row
    state = read(args.assets / "state.json")
    output.mkdir(parents=True)
    public = output / "public"
    payload = output / "encoded"
    payload.mkdir()
    groups = {}
    def encode(item):
        """每张官方 PNG 只编码一次，兼容包和完整包复用输出，避免二次有损压缩。"""
        canonical = item["canonical"]
        name = item["previewFile"]
        if name != canonical + ".webp" or Path(name).name != name:
            raise ValueError("预览名必须保留完整资源名称。")
        group = definitions[item["section"]]
        if canonical in maps:
            meta = maps[canonical]
            raw = (args.assets / "outimg" / (canonical + ".png")).read_bytes()
            if digest(raw) != meta["sha256"]:
                raise ValueError(f"原图摘要不符：{canonical}")
            with Image.open(io.BytesIO(raw)) as source:
                rgba = source.convert("RGBA")
                if rgba.size != (meta["width"], meta["height"]):
                    raise ValueError(f"原图尺寸不符：{canonical}")
                buffer = io.BytesIO()
                # 沿用旧预览质量 50；搜索参数 4 减少编码耗时，不能用全尺寸无损图替代预览。
                rgba.save(buffer, "WEBP", lossless=False, quality=ENCODING["quality"], method=ENCODING["method"])
                encoded = buffer.getvalue()
                with Image.open(io.BytesIO(encoded)) as result:
                    result.load()
                    if result.format != "WEBP" or result.size != rgba.size:
                        raise ValueError(f"预览格式或尺寸不符：{canonical}")
                    if result.convert("RGBA").getchannel("A").tobytes() != rgba.getchannel("A").tobytes():
                        raise ValueError(f"预览透明度不符：{canonical}")
                size = list(rgba.size)
            source_kind = "official-extraction"
            source_hash = meta["sha256"]
        elif name in old:
            encoded = old[name]
            with Image.open(io.BytesIO(encoded)) as image:
                image.load()
                size = list(image.size)
            source_kind = "retained-legacy-preview"
            source_hash = digest(encoded)
        else:
            raise ValueError(f"资源没有已核验的图片来源：{canonical}")
        (payload / name).write_bytes(encoded)
        return {"name": name, "group": group, "source": source_kind,
                "sourceSha256": source_hash, "sha256": digest(encoded), "size": size, "bytes": len(encoded),
                "change": "added" if name not in old else "unchanged" if old[name] == encoded else "updated"}

    records = []
    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        for record in executor.map(encode, catalog):
            records.append(record)
            groups.setdefault(record["group"], []).append(record["name"])
            if len(records) % 100 == 0:
                print(f"已核验并编码 {len(records)}/{len(catalog)}", flush=True)

    def pack(key, names, relative):
        destination = public / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        expanded = 0
        with zipfile.ZipFile(destination, "x", compression=zipfile.ZIP_STORED) as archive:
            for name in sorted(names):
                data = (payload / name).read_bytes()
                if len(data) > 16 * 1024 * 1024:
                    raise ValueError("单张预览超过客户端限制。")
                expanded += len(data)
                entry = zipfile.ZipInfo("previews/" + name, (2026, 11, 15, 0, 0, 0))
                entry.external_attr = 0o100644 << 16
                archive.writestr(entry, data)
        raw = destination.read_bytes()
        if len(raw) > 256 * 1024 * 1024 or expanded > 512 * 1024 * 1024:
            raise ValueError("预览包超过客户端限制。")
        return {"key": key, "version": args.version, "bundleUrl": args.raw_prefix + relative,
                "sha256": digest(raw), "bytes": len(raw), "files": len(names)}

    modern = {"schema": 2, "bundles": [pack(key, names, f"bundles/preview-v{args.version}/APMM-previews-{key}-{args.version}.zip")
                                         for key, names in sorted(groups.items())]}
    legacy = {"schema": 2, "bundles": [pack(key, names, f"bundles/astral-party-previews-{key}-preview-v{args.version}-compat.zip")
                                         for key, names in sorted(old_groups.items())]}
    # 旧客户端按 GitHub 资产名构造 CNB 链接，兼容包必须保留平铺文件名。
    for bundle in legacy["bundles"]:
        filename = bundle["bundleUrl"].rsplit("/", 1)[-1]
        bundle["bundleUrl"] = f"https://github.com/a190141074/AstralPartyModManager-Previews/releases/download/preview-v{args.version}/{filename}"
    for name, value in [("preview-manifest-v4.json", modern), ("preview-manifest.json", legacy)]:
        (public / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    report = {"previewVersion": args.version, "gameVersion": state.get("accepted_version"), "encoding": ENCODING,
              "catalogFingerprint": state.get("accepted_catalog_fingerprint"),
              "total": len(records), "official": sum(r["source"] == "official-extraction" for r in records),
              "retainedLegacy": [r["name"] for r in records if r["source"] != "official-extraction"],
              "added": sum(r["change"] == "added" for r in records),
              "updated": sum(r["change"] == "updated" for r in records),
              "records": records, "manifests": {"modern": modern, "legacy": legacy}}
    (output / "preview-build.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({key: value for key, value in report.items() if key not in {"records", "manifests", "retainedLegacy"}}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--legacy-manifest", type=Path, required=True)
    parser.add_argument("--legacy-bundles", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--version", required=True)
    parser.add_argument("--raw-prefix", required=True)
    options = parser.parse_args()
    if not options.version.isdigit() or not options.raw_prefix.endswith("/"):
        parser.error("版本必须是整数，raw-prefix 必须以 / 结尾。")
    if not 1 <= options.workers <= 8:
        parser.error("workers 必须在 1 到 8 之间。")
    build(options)
