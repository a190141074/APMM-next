"""用途：提供容器与纹理的合成对象。作者：待填写。修改记录：2026-11-14，新增精确引用测试夹具。"""

from types import SimpleNamespace

from apmm_util.models import AssetSpec, BundleSpec


class FakeReader:
    """模拟小型对象读取器，避免测试依赖游戏资产。"""

    def __init__(self, kind, identifier, source, tree):
        """功能：创建合成读取器。入参：类型、对象号、来源和字段。返回：读取器。异常：无。"""
        self.type = SimpleNamespace(name=kind)
        self.path_id, self.assets_file, self.tree = identifier, source, tree

    def read_typetree(self):
        """功能：返回合成类型树。入参：无。返回：字段字典。异常：无。"""
        return self.tree

    def peek_name(self):
        """功能：读取对象名称。入参：无。返回：名称。异常：无。"""
        return self.tree.get("m_Name", "")

    def read(self):
        """功能：构造字段对象。入参：无。返回：模拟纹理。异常：无。"""
        fields = dict(self.tree)
        if isinstance(fields.get("m_StreamData"), dict):
            fields["m_StreamData"] = SimpleNamespace(**fields["m_StreamData"])
        return SimpleNamespace(**fields)


def texture_session(name="UT_Hero_Bust_108_02_1_sfw", identifier=-8858869715767140405):
    """功能：构造准确容器和纹理。入参：完整名称和对象号。返回：资产、会话和包。异常：无。"""
    source = SimpleNamespace(name="CAB-test", externals=[], objects={})
    texture = FakeReader("Texture2D", identifier, source, {"m_Name": name, "m_Width": 2, "m_Height": 2,
                         "m_TextureFormat": 4, "m_ImageCount": 1, "m_TextureDimension": 2,
                         "m_StreamData": {"offset": 0, "size": 16, "path": "archive:/CAB-test/CAB-test.resS"}})
    container = FakeReader("AssetBundle", 1, source, {"m_Name": "bundle", "m_Container": [
                           ("7", {"asset": {"m_FileID": 0, "m_PathID": identifier}})]})
    source.objects = {1: container, identifier: texture}
    session = SimpleNamespace(files={"cab-test": source}, streams={"cab-test.ress": bytes(range(16))},
                              loaded_bytes=1024, warnings=[])
    spec = BundleSpec("remote/bundle", "bundle", "a" * 32)
    bundle = SimpleNamespace(spec=spec)
    return AssetSpec(name, "7", "UnityEngine.Texture2D", 1, (spec,)), session, (bundle,)
