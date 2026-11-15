"""用途：构造最小目录与 UnityFS 测试数据。作者：待填写。修改记录：2026-11-14，首次创建。"""

import base64
import json
import struct
from pathlib import Path


def compact_catalog(names=("UT_HandCard_10001", "UT_HandCard_20006_sfw")) -> dict:
    """功能：构造含手牌和依赖的最小目录。入参：资源名序列。返回：官方格式目录。异常：无。"""
    bundle_key = "bundle.bundle"
    keys = [bundle_key, *names]
    key_data, bucket_data = bytearray(struct.pack("<i", len(keys))), bytearray(struct.pack("<i", len(keys)))
    for i, name in enumerate(keys):
        offset = len(key_data)
        kind = 1 if i == 2 else 0
        encoded = name.encode("utf-16-le" if kind else "ascii")
        key_data.extend(bytes([kind]) + struct.pack("<i", len(encoded)) + encoded)
        bucket_data.extend(struct.pack("<iii", offset, 1, i))
    options = json.dumps({"m_BundleName": "bundle", "m_Hash": "a" * 32}).encode("utf-16-le")
    extra = b"\x07\x01A\x01B" + struct.pack("<i", len(options)) + options
    entries = [(0, 0, -1, 0, 0, 0, 0)]
    for i, _ in enumerate(names, 1):
        entries.append((i, 1, 0, 0, -1, i, 1))
    encode = lambda data: base64.b64encode(data).decode()
    return {"m_InternalIds": ["{App.WebServerConfig.Path}/bundle.bundle", *map(str, range(1, len(keys)))],
            "m_ProviderIds": ["UnityEngine.ResourceManagement.ResourceProviders.AssetBundleProvider",
                              "UnityEngine.ResourceManagement.ResourceProviders.BundledAssetProvider"],
            "m_resourceTypes": [{"m_ClassName": "IAssetBundleResource"}, {"m_ClassName": "UnityEngine.Texture2D"}],
            "m_KeyDataString": encode(key_data), "m_BucketDataString": encode(bucket_data),
            "m_EntryDataString": encode(struct.pack("<i", len(entries)) + b"".join(struct.pack("<7i", *e) for e in entries)),
            "m_ExtraDataString": encode(extra), "m_InternalIdPrefixes": []}


def unityfs(payload: bytes, declared_size=None) -> bytes:
    """功能：构造未压缩 UnityFS。入参：节点内容和可选旧长度。返回：资源包。异常：无。"""
    size = len(payload) if declared_size is None else declared_size
    info = (b"\0" * 16 + struct.pack(">I", 1) + struct.pack(">IIH", size, len(payload), 0)
            + struct.pack(">IQQI", 1, 0, size, 4) + b"CAB-test\0")
    prefix = b"UnityFS\0" + struct.pack(">I", 8) + b"5.x.x\0" + b"2022.3.62f3\0"
    header_length = (len(prefix) + 20 + 15) // 16 * 16
    header = prefix + struct.pack(">QIII", header_length + len(info) + len(payload), len(info), len(info), 0x40)
    return header.ljust(header_length, b"\0") + info + payload


def fake_game(base: Path) -> tuple[Path, Path]:
    """功能：构造无真实游戏依赖的目录。入参：测试根目录。返回：游戏及 LocalLow 路径。异常：写入失败。"""
    game, local = base / "game", base / "local"
    aa = game / "Test_Data/StreamingAssets/aa"
    aa.mkdir(parents=True)
    (game / "Test_Data/app.info").write_text("公司\n游戏", encoding="utf-8")
    (aa / "settings.json").write_text(json.dumps({"m_CatalogLocations": [{"m_InternalId": "catalog_3.3.0.hash"}]}), encoding="utf-8")
    cached = local / "com.unity.addressables"
    cached.mkdir(parents=True)
    (cached / "catalog_3.3.0.json").write_text(json.dumps(compact_catalog()), encoding="utf-8")
    (cached / "catalog_3.3.0.hash").write_text("a" * 32, encoding="utf-8")
    return game, local

