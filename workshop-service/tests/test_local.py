"""本机收稿链测试，既不启动服务，也不发布到远端。"""
import base64
import hashlib
import json
import os
import unittest
import uuid
import zipfile
from pathlib import Path
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from workshop_service.local_review import receive, prepare
from workshop_service.publisher import catalog
from workshop_service.models import Metadata
from test_http import package


class LocalTests(unittest.TestCase):
    def setUp(self):
        self.root=Path(os.environ["APMM_WORK_DIR"])/"local-tests"/str(uuid.uuid4())
        self.root.mkdir(parents=True)
        self.key=Ed25519PrivateKey.generate()
        self.allowed={"UT_HandCard_10001"}
        self.work=str(uuid.uuid4())

    def source(self, **kwargs):
        path=self.root/(str(uuid.uuid4())+".zip")
        def identity(meta, entries):
            meta.update(workId=self.work, submissionId=str(uuid.uuid4()))
        path.write_bytes(package(invalid=identity, **kwargs))
        return path

    def review(self, source):
        path=self.root/str(uuid.uuid4())
        report=receive(source,path,self.allowed)
        evidence=json.loads((path/"evidence-template.json").read_text())
        evidence.update(scanResult="synthetic fixture", gameVersion="fixture", testEvidence="synthetic only, no live game",
                        reviewNote="test",authorVerification="fixture source controlled by test")
        file=path/"evidence.json"; file.write_text(json.dumps(evidence),encoding="utf-8")
        return path,file,report

    def test_offline_publish_update_and_reject_stale_evidence(self):
        review,evidence,report=self.review(self.source())
        empty=self.root/"empty"; empty.mkdir()
        out=self.root/"prepared"
        result=prepare(review,evidence,empty,out,self.key,self.allowed)
        self.assertEqual(result["state"],"prepared")
        self.assertFalse((out/"public-verified.json").exists())
        self.assertEqual(list(empty.iterdir()),[])
        work=catalog(out/"workshop",self.key)[0]
        self.assertEqual(work["revision"],1)
        self.assertFalse((out/"workshop"/"evidence.json").exists())
        update,proof,_=self.review(self.source(metadata_only=True,revision=1,version="1.1"))
        second=self.root/"update"
        prepare(update,proof,out/"workshop",second,self.key,self.allowed)
        next_work=catalog(second/"workshop",self.key)[0]
        self.assertEqual(next_work["archive"],work["archive"])
        self.assertEqual(next_work["revision"],2)
        with self.assertRaises(ValueError):
            prepare(review,evidence,second/"workshop",self.root/"stale",self.key,self.allowed)
        evidence.write_text(json.dumps({**json.loads(evidence.read_text()),"snapshotHash":"0"*64}))
        with self.assertRaises(ValueError):
            prepare(review,evidence,empty,self.root/"mismatch",self.key,self.allowed)

    def test_author_evidence_required_and_duplicate_names_independent(self):
        review,evidence,_=self.review(self.source(names=["同名.png","同名.png"]))
        proof=json.loads(evidence.read_text());proof["authorVerification"]=" "
        evidence.write_text(json.dumps(proof))
        empty=self.root/"empty";empty.mkdir()
        with self.assertRaises(ValueError):
            prepare(review,evidence,empty,self.root/"rejected",self.key,self.allowed)
        proof["authorVerification"]="test author verified";evidence.write_text(json.dumps(proof))
        prepare(review,evidence,empty,self.root/"ok",self.key,self.allowed)
        work=catalog(self.root/"ok"/"workshop",self.key)[0]
        self.assertEqual(len(work["files"]),2)
        self.assertNotEqual(work["files"][0]["name"],work["files"][1]["name"])

    def test_actual_rust_export_and_signed_catalog_fixture(self):
        path=os.environ.get("APMM_WORKSHOP_EXPORT_FIXTURE")
        if not path:self.skipTest("需要本轮 Rust 离线导出夹具")
        review,evidence,_=self.review(Path(path))
        # 为双协议测试添加封面，内容 PNG 保持原始字节。
        source=self.source()
        with zipfile.ZipFile(Path(path)) as archive:
            meta=json.loads(archive.read("submission.json"))
            original=archive.read(meta["files"][0]["name"])
        with zipfile.ZipFile(source) as archive:
            legacy_meta=json.loads(archive.read("submission.json"))
        self.assertEqual(hashlib.sha256(original).hexdigest(),meta["files"][0]["sha256"])
        empty=self.root/"empty";empty.mkdir()
        out=self.root/"rust-prepared"
        prepare(review,evidence,empty,out,self.key,self.allowed)
        work=catalog(out/"workshop",self.key)[0]
        with zipfile.ZipFile(out/"workshop"/work["archive"]) as archive:
            self.assertEqual(archive.read(work["files"][0]["name"]),original)
        self.assertEqual(work["artist"], "画师测试")
        self.assertEqual(work["authorUrls"], ["https://example.com/author"])
        self.assertEqual(work["artistUrls"], ["https://example.com/artist"])
        self.assertEqual(work["sourceUrl"], "https://example.com/source")
        self.assertIn("\n", work["author"])
        with zipfile.ZipFile(Path(path)) as archive:
            self.assertEqual((out/"workshop"/work["cover"]).read_bytes(), archive.read(meta["images"][0]["name"]))
        review,evidence,_=self.review(source)
        dual=self.root/"dual"
        prepare(review,evidence,empty,dual,self.key,self.allowed)
        fixture=Path(os.environ["APMM_INTEROP_FIXTURE"])
        fixture.write_text(json.dumps({"public":str(dual/"workshop"),"key":base64.b64encode(self.key.public_key().public_bytes_raw()).decode()}))

    def test_old_metadata_and_public_link_boundaries(self):
        old = Metadata(title="旧作品", description="简介", version="1")
        self.assertEqual(old.authorUrls, [])
        self.assertEqual(old.artist, "")
        for url in ["http://example.com", "javascript:alert(1)", "https://user:pass@example.com", "https://example.com/\npath", ""]:
            with self.assertRaises(ValueError):
                Metadata(title="作品", description="简介", version="1", authorUrls=[url])
        with self.assertRaises(ValueError):
            Metadata(title="作品", description="简介", version="1", artistUrls=["https://example.com"] * 9)

    def test_metadata_only_preserves_images_and_can_replace_cover(self):
        review,evidence,_=self.review(self.source())
        empty=self.root/"empty"; empty.mkdir()
        first=self.root/"first"
        prepare(review,evidence,empty,first,self.key,self.allowed)
        previous=catalog(first/"workshop",self.key)[0]
        def update(meta, entries):
            meta.update(workId=self.work, submissionId=str(uuid.uuid4()), images=[])
            meta["metadata"].update(artist="新画师", authorUrls=["https://example.com/author"])
            entries.pop("cover.png")
        source=self.root/"metadata.zip"
        source.write_bytes(package(metadata_only=True, revision=1, invalid=update))
        review,evidence,_=self.review(source)
        second=self.root/"second"
        prepare(review,evidence,first/"workshop",second,self.key,self.allowed)
        current=catalog(second/"workshop",self.key)[0]
        self.assertEqual(current["cover"], previous["cover"])
        self.assertEqual(current["archive"], previous["archive"])
        self.assertEqual(current["artist"], "新画师")
        def replacement(meta, entries):
            from test_http import png
            meta.update(workId=self.work, submissionId=str(uuid.uuid4()))
            data=png("green"); entries["cover.png"]=data
            meta["images"][0].update(sha256=hashlib.sha256(data).hexdigest(), bytes=len(data))
        source=self.root/"replace.zip"
        source.write_bytes(package(metadata_only=True, revision=2, invalid=replacement))
        review,evidence,_=self.review(source)
        third=self.root/"third"
        prepare(review,evidence,second/"workshop",third,self.key,self.allowed)
        final=catalog(third/"workshop",self.key)[0]
        self.assertNotEqual(final["cover"], current["cover"])
        self.assertEqual(final["archive"], current["archive"])


if __name__=="__main__":unittest.main()
