"""无服务器的本地收稿与发布准备；从不推送 Git，也不自动发出消息。"""
from __future__ import annotations
import base64
import json
import shutil
import uuid
from pathlib import Path
from pydantic import Field, field_validator
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from .inspection import inspect, digest
from .models import Evidence, MAX_UPLOAD
from .publisher import build, catalog, encoded


class LocalEvidence(Evidence):
    authorVerification: str = Field(min_length=1, max_length=4000)

    @field_validator("authorVerification")
    @classmethod
    def author_present(cls, value):
        if not value.strip():
            raise ValueError("需记录通过原收稿渠道核实作者身份的依据")
        return value


def new_directory(path: Path):
    path.mkdir(parents=True, exist_ok=False)


def receive(archive: Path, output: Path, allowed: set[str]):
    if archive.stat().st_size > MAX_UPLOAD:
        raise ValueError("投稿包超过 80 MiB")
    new_directory(output)
    copied = output / "submission.zip"
    shutil.copyfile(archive, copied)
    submission = inspect(copied, output / "inspected", allowed)
    report = {"snapshotHash": digest(copied), "submission": submission.model_dump()}
    (output / "review.json").write_bytes(encoded(report))
    evidence = {"snapshotHash": report["snapshotHash"], "scanResult": "", "gameVersion": "",
                "testEvidence": "", "reviewNote": "", "authorVerification": ""}
    (output / "evidence-template.json").write_text(json.dumps(evidence, ensure_ascii=False, indent=2), encoding="utf-8")
    return report


def read_key(path: Path):
    if path.stat().st_size > 128:
        raise ValueError("签名密钥文件大小无效")
    return Ed25519PrivateKey.from_private_bytes(base64.b64decode(path.read_bytes().strip(), validate=True))


def prepare(review: Path, evidence_file: Path, current: Path, output: Path, key, allowed: set[str]):
    """在新目录生成签名工坊；证据与快照绑定，现有公开目录保持只读。"""
    report = json.loads((review / "review.json").read_bytes())
    evidence = LocalEvidence.model_validate_json(evidence_file.read_bytes())
    archive = review / "submission.zip"
    sha = digest(archive)
    if evidence.snapshotHash != sha or report["snapshotHash"] != sha:
        raise ValueError("审核证据与投稿包摘要不符")
    work_id = report["submission"].get("workId")
    submission_id = report["submission"].get("submissionId")
    for identity in (work_id, submission_id):
        if not identity or str(uuid.UUID(identity)) != identity:
            raise ValueError("此旧投稿包缺少稳定编号，请用新版重新导出")
    works = catalog(current, key)
    previous = next((work for work in works if work["id"] == work_id), None)
    revision = previous.get("revision", 0) if previous else 0
    if report["submission"]["baseRevision"] != revision:
        raise ValueError("投稿基线已过期，请作者同步公开版本后重新导出")
    if report["submission"]["metadataOnly"] and not previous:
        raise ValueError("未发布作品不能仅更新资料")
    new_directory(output)
    public = output / "workshop"
    public.mkdir()
    # 仅复制签名目录声明的资源，避免把现有仓库内的私有文件带入待发布目录。
    names = set()
    for work in works:
        names.update([work["archive"], *([work["cover"]] if work.get("cover") else []), *work.get("previews", [])])
    names.update(name for name in ("manifest-v2.json", "manifest.json", "manifest.sig") if (current / name).exists())
    for name in names:
        if Path(name).name != name or ".." in name or ":" in name or "\\" in name:
            raise ValueError("已有目录含非法资源路径")
        source = current / name
        if source.is_symlink() or not source.is_file():
            raise ValueError("已有目录资源缺失或为符号链接")
        shutil.copyfile(source, public / name)
    intent = {"id": submission_id, "workId": work_id, "snapshotHash": sha,
              "submission": report["submission"], "published": previous, "revision": revision + 1,
              "operation": "publish", "lease": str(uuid.uuid4())}
    stage = output / "private"
    stage.mkdir()
    result = build(public, stage, intent, archive, key, allowed)
    summary = {"state": "prepared", "workId": work_id, "submissionId": submission_id,
               "snapshotHash": sha, "catalogHash": result["catalogHash"],
               "baseCatalogHash": base_hash(current),
               "files": {path.name: digest(path) for path in public.iterdir() if path.is_file()},
               "publicKey": base64.b64encode(key.public_key().public_bytes_raw()).decode()}
    (output / "prepared.json").write_bytes(encoded(summary))
    (stage / "evidence.json").write_bytes(encoded(evidence.model_dump()))
    return summary


def base_hash(current: Path):
    for name in ("manifest-v2.json", "manifest.json"):
        if (current / name).exists():
            return digest(current / name)
    return None


def check_base(prepared: Path, current: Path):
    """复制到资产仓库前检查基线，防止覆盖期间新增的其他作品。"""
    report = json.loads((prepared / "prepared.json").read_bytes())
    if report["baseCatalogHash"] != base_hash(current):
        raise ValueError("资产仓库目录已更新，请保留审核资料并重新准备发布目录")
    for name, sha in report["files"].items():
        if Path(name).name != name or ".." in name or "\\" in name or digest(prepared / "workshop" / name) != sha:
            raise ValueError("待发布文件已改变")


def verify_public(prepared: Path, base: str):
    """公开资源逐项匹配后记录确认结果；没有网络发布写入副作用。"""
    import hashlib
    import httpx
    from urllib.parse import urlparse
    url = urlparse(base)
    if (url.scheme != "https" or url.netloc != "cnb.cool" or url.query or url.fragment
        or url.path.rstrip("/") != "/astral-weird-egg/astral-party-mod-manager-assets/-/git/raw/main/workshop"):
        raise ValueError("需要应用固定的 CNB 工坊公开地址")
    report = json.loads((prepared / "prepared.json").read_bytes())
    with httpx.Client(timeout=60, follow_redirects=False) as client:
        for name, sha in report["files"].items():
            if Path(name).name != name or ".." in name or "\\" in name:
                raise ValueError("待发布清单路径无效")
            local = prepared / "workshop" / name
            if digest(local) != sha:
                raise ValueError("待发布目录已改变")
            count = 0
            actual = hashlib.sha256()
            with client.stream("GET", base.rstrip("/") + "/" + name) as response:
                response.raise_for_status()
                for chunk in response.iter_bytes():
                    count += len(chunk)
                    if count > local.stat().st_size:
                        raise ValueError("公开文件长度不符")
                    actual.update(chunk)
            if actual.hexdigest() != sha:
                raise ValueError("公开文件尚未匹配，请保持待发布状态并稍后复核")
    (prepared / "public-verified.json").write_bytes(encoded({"state":"public_verified", "catalogHash":report["catalogHash"]}))

