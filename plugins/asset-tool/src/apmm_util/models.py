"""用途：定义目录、定位和同步的公共类型。作者：待填写。修改记录：2026-11-14，首次创建。"""

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class Config:
    """保存已解析的运行路径与资源预算。"""

    game_dir: Path
    data_dir: Path
    locallow_dir: Path
    cache_dirs: tuple[Path, ...]
    output_dir: Path
    memory_budget: int = 256 * 1024 * 1024
    delay_ms: int = 20


@dataclass(frozen=True)
class FileStamp:
    """保存文件身份，检测替换和写入中的目录。"""

    size: int
    modified_ns: int
    identity: int


@dataclass(frozen=True)
class CatalogSnapshot:
    """保存当前目录的来源、版本及一致性校验依据。"""

    path: Path
    hash_path: Path | None
    version: str
    catalog_hash: str
    content_sha256: str
    fingerprint: str
    stamp: FileStamp
    kind: str
    warnings: tuple[str, ...] = ()
    settings_path: Path | None = None
    settings_stamp: FileStamp | None = None
    pending_cache_paths: tuple[Path, ...] = ()


@dataclass(frozen=True)
class BundleSpec:
    """保存官方目录提供的资源包名称、版本和位置。"""

    internal_id: str
    name: str
    version_hash: str
    crc: int = 0


@dataclass(frozen=True)
class AssetSpec:
    """保存完整资源名、容器键和依赖资源包。"""

    name: str
    internal_id: str
    resource_type: str
    location_index: int
    bundles: tuple[BundleSpec, ...]


@dataclass(frozen=True)
class LocatedBundle:
    """保存已定位资源包及其内容指纹。"""

    spec: BundleSpec
    path: Path
    sha256: str
    size: int
    stamp: FileStamp


@dataclass
class ExtractedImage:
    """保存解码图像及可追踪的对象标识。"""

    image: Any
    object_id: int
    serialized_file: str
    texture_format: int
    warnings: list[str] = field(default_factory=list)

