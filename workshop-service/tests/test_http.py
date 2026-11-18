"""真实回环 HTTP、SQLite 重启及签名本地仓库的隔离验收。"""
from __future__ import annotations
import base64
import hashlib
import io
import json
import os
import secrets
import socket
import subprocess
import sys
import threading
import time
import unittest
import uuid
import zipfile
from pathlib import Path
import httpx
import uvicorn
from PIL import Image
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization
from workshop_service.api import create_app
from workshop_service.publisher import catalog

def png(color="red"):
    out=io.BytesIO();Image.new("RGBA",(2,2),color).save(out,format="PNG");return out.getvalue()

def package(*,names=None, metadata_only=False, revision=0, version="1.0", invalid=None):
    data=png();files=[];entries={}
    for index,name in enumerate(names if names is not None else ["UT_HandCard_10001.png"]):
        if metadata_only:break
        key=f"file-{index}.png";content=png("red" if index==0 else "blue")
        files.append({"name":key,"sourceName":name,"canonical":"UT_HandCard_10001","sha256":hashlib.sha256(content).hexdigest(),"bytes":len(content)})
        entries[key]=content
    entries["cover.png"]=data
    meta={"schema":1,"metadata":{"title":"测试作品","author":"作者","description":"隔离测试","category":"卡面","tags":["测试"],"version":version,"gameVersion":"test-game","changeNote":"测试记录"},
          "metadataOnly":metadata_only,"baseRevision":revision,"files":files,"images":[{"name":"cover.png","sourceName":"cover.png","canonical":"","sha256":hashlib.sha256(data).hexdigest(),"bytes":len(data)}]}
    if invalid:invalid(meta,entries)
    out=io.BytesIO()
    with zipfile.ZipFile(out,"w",compression=zipfile.ZIP_DEFLATED) as zip:
        zip.writestr("submission.json",json.dumps(meta).encode())
        for name,content in entries.items():zip.writestr(name,content)
    return out.getvalue()

class HttpTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root=Path(os.environ["APMM_WORK_DIR"])/"service-tests"/str(uuid.uuid4());cls.root.mkdir(parents=True)
        cls.index=cls.root/"index.json";cls.index.write_text('["UT_HandCard_10001"]')
        cls.admin=secrets.token_urlsafe(32)
        cls.start()

    @classmethod
    def start(cls):
        sock=socket.socket();sock.bind(("127.0.0.1",0));port=sock.getsockname()[1];sock.close()
        app=create_app(cls.root/"private",cls.admin,cls.index)
        cls.server=uvicorn.Server(uvicorn.Config(app,host="127.0.0.1",port=port,log_level="critical",access_log=False))
        cls.thread=threading.Thread(target=cls.server.run,daemon=True);cls.thread.start();cls.base=f"http://127.0.0.1:{port}"
        for _ in range(100):
            if cls.server.started:break
            time.sleep(.02)
        if not cls.server.started:raise RuntimeError("测试服务未启动")

    @classmethod
    def stop(cls):
        cls.server.should_exit=True;cls.thread.join(5)
        if cls.thread.is_alive():raise RuntimeError("测试服务未退出")

    @classmethod
    def tearDownClass(cls):cls.stop()

    def setUp(self):
        self.work=str(uuid.uuid4());self.token=secrets.token_urlsafe(32)
        self.api=httpx.Client(base_url=self.base,headers={"Authorization":"Bearer "+self.token},timeout=10)
        self.admin_api=httpx.Client(base_url=self.base,headers={"Authorization":"Bearer "+self.admin},timeout=10)
        self.assertEqual(self.api.put(f"/v1/works/{self.work}").status_code,200)

    def tearDown(self):self.api.close();self.admin_api.close()

    def upload(self,data=None,id=None):
        data=data if data is not None else package();id=id or str(uuid.uuid4())
        reply=self.api.put(f"/v1/works/{self.work}/submissions/{id}",headers={"X-Content-SHA256":hashlib.sha256(data).hexdigest()},content=data)
        return id,data,reply

    def evidence(self,data):
        return {"snapshotHash":hashlib.sha256(data).hexdigest(),"scanResult":"fixture scan only","gameVersion":"fixture-game","testEvidence":"isolated fixture, not live game","reviewNote":"test only"}

    def approve(self,id,data):
        self.assertEqual(self.admin_api.post(f"/v1/admin/submissions/{id}/approve",json=self.evidence(data)).status_code,200)

    def repo(self):
        repo=self.root/str(uuid.uuid4());repo.mkdir()
        subprocess.run(["git","init","-b","main",str(repo)],check=True,capture_output=True)
        subprocess.run(["git","-C",str(repo),"-c","user.name=Test","-c","user.email=test@invalid.local","commit","--allow-empty","-m","fixture"],check=True,capture_output=True)
        return repo

    def publish(self,id,repo,key,operation="publish"):
        env={**os.environ,"APMM_REVIEW_URL":self.base,"APMM_REVIEW_TOKEN":self.admin,
             "APMM_PUBLISH_PRIVATE_KEY":base64.b64encode(key.private_bytes(serialization.Encoding.Raw,serialization.PrivateFormat.Raw,serialization.NoEncryption())).decode()}
        command=[sys.executable,str(Path(__file__).parents[1]/"tools/publish.py"),id,"--repo",str(repo),"--stage",str(self.root/"publish-stage"/id),"--index",str(self.index),"--operation",operation]
        result=subprocess.run(command,env=env,capture_output=True,timeout=25)
        self.assertEqual(result.returncode,0,result.stderr.decode(errors="replace"))

    def test_01_authorization_idempotence_and_snapshot_conflict(self):
        id,data,response=self.upload();self.assertEqual(response.status_code,200)
        self.assertEqual(self.upload(data,id)[2].json(),response.json())
        self.assertEqual(self.upload(package(version="2.0"),id)[2].status_code,409)
        other=httpx.get(self.base+f"/v1/works/{self.work}",headers={"Authorization":"Bearer "+secrets.token_urlsafe(32)})
        self.assertEqual(other.status_code,403)
        self.assertNotIn(self.token,response.text)
        self.assertEqual(self.api.get(f"/v1/admin/submissions/{id}/archive").status_code,403)

    def test_02_bad_archives_and_classification(self):
        for change in [lambda m,e:e.update({"../escape.png":png()}),lambda m,e:e.update({"file-0.png":b"fake png"}),
                       lambda m,e:m["files"][0].update(canonical="unknown_slot"),
                       lambda m,e:m["images"][0].update(bytes=3*1024**2)]:
            self.assertEqual(self.upload(package(invalid=change))[2].status_code,422)
        self.assertFalse((self.root/"private"/"escape.png").exists())

    def test_03_disconnect_and_same_id_retry(self):
        data=package();id=str(uuid.uuid4());sha=hashlib.sha256(data).hexdigest()
        port=int(self.base.rsplit(":",1)[1]);connection=socket.create_connection(("127.0.0.1",port))
        header=f"PUT /v1/works/{self.work}/submissions/{id} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {self.token}\r\nX-Content-SHA256: {sha}\r\nContent-Length: {len(data)}\r\n\r\n".encode()
        connection.sendall(header+data[:20]);connection.close();time.sleep(.1)
        self.assertEqual(self.upload(data,id)[2].status_code,200)
        self.assertEqual(len(self.api.get(f"/v1/works/{self.work}").json()["submissions"]),1)

    def test_04_evidence_publication_failure_restart_and_withdraw(self):
        id,data,response=self.upload();self.assertEqual(response.status_code,200)
        self.assertEqual(self.admin_api.post(f"/v1/admin/submissions/{id}/approve",json={}).status_code,422)
        wrong=self.evidence(data);wrong["snapshotHash"]="0"*64
        self.assertEqual(self.admin_api.post(f"/v1/admin/submissions/{id}/approve",json=wrong).status_code,409)
        self.approve(id,data)
        begin=self.admin_api.post(f"/v1/admin/submissions/{id}/begin",json={"operation":"publish"}).json()
        self.assertFalse(begin["done"])
        self.assertIsNone(self.api.get(f"/v1/works/{self.work}").json()["published"])
        self.api.close();self.admin_api.close();type(self).stop();type(self).start()
        self.api=httpx.Client(base_url=self.base,headers={"Authorization":"Bearer "+self.token},timeout=10)
        self.admin_api=httpx.Client(base_url=self.base,headers={"Authorization":"Bearer "+self.admin},timeout=10)
        retry=self.admin_api.post(f"/v1/admin/submissions/{id}/begin",json={"operation":"publish"}).json()
        self.assertEqual(retry["lease"],begin["lease"])
        repo=self.repo();key=Ed25519PrivateKey.generate();self.publish(id,repo,key)
        current=self.api.get(f"/v1/works/{self.work}").json()
        self.assertEqual(current["revision"],1);self.assertEqual(current["submissions"][0]["status"],"published")
        before=(repo/"workshop/manifest-v2.json").read_bytes();self.publish(id,repo,key)
        self.assertEqual((repo/"workshop/manifest-v2.json").read_bytes(),before)
        works=catalog(repo/"workshop",key);self.assertEqual(len(works),1)
        legacy=json.loads((repo/"workshop/manifest.json").read_bytes());self.assertEqual(len(legacy["works"]),1)
        self.publish(id,repo,key,"withdraw")
        self.assertEqual(catalog(repo/"workshop",key),[])
        self.assertEqual(json.loads((repo/"workshop/manifest.json").read_bytes())["works"],[])

    def test_05_duplicate_names_metadata_update_and_rejection(self):
        id,data,response=self.upload(package(names=["同名.png","同名.png"]))
        self.assertEqual(response.status_code,200);self.approve(id,data)
        repo=self.repo();key=Ed25519PrivateKey.generate();self.publish(id,repo,key)
        work=catalog(repo/"workshop",key)[0];self.assertEqual(len(work["files"]),2)
        self.assertEqual(len({f["name"] for f in work["files"]}),2)
        self.assertEqual(json.loads((repo/"workshop/manifest.json").read_bytes())["works"],[])
        next_id,next_data,response=self.upload(package(metadata_only=True,revision=1,version="1.1"));self.assertEqual(response.status_code,200)
        self.approve(next_id,next_data);self.publish(next_id,repo,key)
        self.assertEqual(catalog(repo/"workshop",key)[0]["archive"],work["archive"])
        rejected,data,response=self.upload(package(revision=2,version="2.0"));self.assertEqual(response.status_code,200)
        self.assertEqual(self.admin_api.post(f"/v1/admin/submissions/{rejected}/reject",json=self.evidence(data)).status_code,200)
        retry_id,_,response=self.upload(package(revision=2,version="2.1"));self.assertEqual(response.status_code,200)
        self.assertEqual(self.api.delete(f"/v1/works/{self.work}/submissions/{retry_id}").status_code,200)
        self.assertEqual(self.api.delete(f"/v1/works/{self.work}/submissions/{id}").status_code,409)

    def test_06_issue_does_not_include_credentials_paths_or_attachments(self):
        id,_,response=self.upload();self.assertEqual(response.status_code,200)
        output=self.root/"issue.md"
        env={**os.environ,"APMM_REVIEW_URL":self.base,"APMM_REVIEW_TOKEN":self.admin}
        result=subprocess.run([sys.executable,str(Path(__file__).parents[1]/"tools/review.py"),"issue",id,"--output",str(output)],env=env,capture_output=True)
        self.assertEqual(result.returncode,0)
        text=output.read_text(encoding="utf-8")
        for secret in [self.token,self.admin,str(self.root),"/archive","Authorization"]:self.assertNotIn(secret,text)

if __name__=="__main__":unittest.main(verbosity=2)
