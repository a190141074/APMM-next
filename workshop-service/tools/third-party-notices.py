"""从锁定虚拟环境的安装元数据生成维护工具依赖归属与许可原文。"""
import importlib.metadata
from pathlib import Path

root = Path(__file__).resolve().parents[1]
sections = ["# 本机审核工具与可选服务的第三方声明\n\n生成源：requirements.lock 与对应安装环境。桌面便携包不包含这些 Python 依赖。\n"]
for line in (root / "requirements.lock").read_text(encoding="utf-8").splitlines():
    if "==" not in line or line.startswith("#"):
        continue
    name, version = line.split("==", 1)
    distribution = importlib.metadata.distribution(name)
    if distribution.version != version:
        raise ValueError("安装环境与锁定依赖不一致：" + name)
    license_name = distribution.metadata.get("License-Expression") or distribution.metadata.get("License") or "见下方许可原文"
    texts = []
    for entry in distribution.files or []:
        if entry.name.lower().startswith(("license", "licence", "copying", "notice")):
            path = Path(distribution.locate_file(entry))
            if path.is_file():
                texts.append("### " + str(entry) + "\n\n" + path.read_text(encoding="utf-8", errors="replace"))
    if not texts:
        raise ValueError("未找到依赖许可原文：" + name)
    sections.append(f"## {name} {version}\n\n声明许可：{license_name}\n\n" + "\n\n".join(texts))
(root / "THIRD_PARTY_NOTICES.md").write_text("\n\n".join(sections) + "\n", encoding="utf-8")
