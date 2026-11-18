"""发布环境专用目录生成器，资源先写入不可变内容地址，再原子切换签名响应。"""
from __future__ import annotations
import base64
import hashlib
import json
import os
import time
import zipfile
from pathlib import Path
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from .inspection import digest, inspect

def encoded(value):
    return json.dumps(value,ensure_ascii=False,separators=(",",":"),sort_keys=True).encode()

def atomic(path: Path, data: bytes):
    temp=path.with_suffix(path.suffix+".pending")
    with temp.open("wb") as out:
        out.write(data); out.flush(); os.fsync(out.fileno())
    os.replace(temp,path)

def immutable(public: Path, data: bytes, suffix: str):
    sha=hashlib.sha256(data).hexdigest()
    path=public/f"{sha}{suffix}"
    if path.exists():
        if digest(path)!=sha:
            raise ValueError("现有不可变资源已损坏")
    else:
        atomic(path,data)
    if digest(path)!=sha:
        raise ValueError("发布资源复核失败")
    return path.name,sha,len(data)

def catalog(public: Path,key):
    v2=public/"manifest-v2.json"
    if v2.exists():
        envelope=json.loads(v2.read_bytes())
        data=base64.b64decode(envelope["payload"],validate=True)
        key.public_key().verify(base64.b64decode(envelope["signature"],validate=True),data)
        value=json.loads(data)
    elif (public/"manifest.json").exists():
        data=(public/"manifest.json").read_bytes()
        key.public_key().verify((public/"manifest.sig").read_bytes(),data)
        value=json.loads(data)
    else:
        value={"schema":2,"works":[]}
    if value.get("schema") not in (1,2) or not isinstance(value.get("works"),list):
        raise ValueError("已有目录格式无效")
    return value["works"]

def build(public: Path, stage: Path, intent, archive: Path, key: Ed25519PrivateKey, allowed: set[str]):
    public.mkdir(parents=True,exist_ok=True)
    works=catalog(public,key)
    if intent["operation"]=="withdraw":
        work=None
        works=[entry for entry in works if entry["id"]!=intent["workId"]]
    else:
        if digest(archive)!=intent["snapshotHash"]:
            raise ValueError("待审包与批准摘要不符")
        meta=inspect(archive,stage,allowed)
        if meta.model_dump()!=intent["submission"]:
            raise ValueError("待审资料与发布交接不符")
        previous=intent["published"]
        # 重试复用已签名的同一修订，避免发布时间变化导致重复目录提交。
        existing=next((entry for entry in works if entry["id"]==intent["workId"] and entry.get("submissionId")==intent["id"]),None)
        if existing:
            work=existing
        else:
            work={**(previous or {}),**meta.metadata.model_dump(),"id":intent["workId"],"submissionId":intent["id"],
                  "publishedId":None,"revision":intent["revision"],"updatedAt":int(time.time()),
                  "tags":[{"name":tag,"highlight":False} for tag in meta.metadata.tags]}
            if not meta.metadataOnly:
                name,sha,size=immutable(public,(stage/"content.zip").read_bytes(),".zip")
                work.update(archive=name,sha256=sha,bytes=size,files=[file.model_dump() for file in meta.files])
            if meta.images:
                images=[immutable(public,(stage/file.name).read_bytes(),".png")[0] for file in meta.images]
                work.update(cover=images[0],previews=images[1:])
            else:
                work.setdefault("cover","");work.setdefault("previews",[])
            history=list((previous or {}).get("history",[]))[-19:]
            if previous:
                history.append({"version":previous["version"],"changeNote":previous.get("changeNote",""),"publishedAt":previous.get("updatedAt",0)})
            work["history"]=history
            works=[entry for entry in works if entry["id"]!=work["id"]]+[work]
    if len(works)>500:
        raise ValueError("公开目录最多支持 500 件作品")
    works.sort(key=lambda entry:entry["id"])
    v2=encoded({"schema":2,"works":works})
    if len(v2)>2*1024*1024:
        raise ValueError("签名目录超过客户端限制")
    # v1 仅包含可用原名称唯一表达的资源；对应归档也必须改写内部名称。
    old=[]
    for entry in works:
        files=entry["files"]
        names=[file.get("sourceName") or file["name"] for file in files]
        if any(name != f"{file.get('canonical') or name[:-4]}.png" or not name.isascii() for name,file in zip(names,files)) or len({name.casefold() for name in names})!=len(names):
            continue
        if not entry.get("cover"):
            continue
        compatible=stage/f"{entry['id']}-v1.zip"
        with zipfile.ZipFile(public/entry["archive"]) as src,zipfile.ZipFile(compatible,"w",compression=zipfile.ZIP_DEFLATED) as dst:
            for name,file in zip(names,files):
                data=src.read(file["name"])
                if hashlib.sha256(data).hexdigest()!=file["sha256"]:
                    raise ValueError("历史资源摘要无效")
                dst.writestr(zipfile.ZipInfo(name),data,compress_type=zipfile.ZIP_DEFLATED)
        name,sha,size=immutable(public,compatible.read_bytes(),".zip")
        old.append({**entry,"archive":name,"sha256":sha,"bytes":size,"files":[{**file,"name":name} for file,name in zip(files,names)]})
    legacy=encoded({"schema":1,"works":old})
    atomic(public/"manifest.json",legacy)
    atomic(public/"manifest.sig",key.sign(legacy))
    envelope=encoded({"payload":base64.b64encode(v2).decode(),"signature":base64.b64encode(key.sign(v2)).decode()})
    atomic(public/"manifest-v2.json",envelope)
    return {"lease":intent["lease"],"operation":intent["operation"],"catalogHash":hashlib.sha256(envelope).hexdigest(),"work":work}
