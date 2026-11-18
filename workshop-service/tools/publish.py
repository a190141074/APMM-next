"""在受保护发布环境中构建签名目录，经 Git 发布及公开摘要复核后回写状态。"""
import argparse
import base64
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
import httpx
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from workshop_service.publisher import build
from review import client

def git(repo,*args):
    result=subprocess.run(["git","-C",str(repo),*args],capture_output=True,env=os.environ.copy())
    if result.returncode: raise RuntimeError("Git 发布步骤失败")
    return result.stdout.decode().strip()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("id");parser.add_argument("--operation",choices=["publish","withdraw"],default="publish")
    parser.add_argument("--repo",type=Path,required=True);parser.add_argument("--stage",type=Path,required=True)
    parser.add_argument("--index",type=Path,required=True);parser.add_argument("--push",action="store_true")
    parser.add_argument("--public-base");args=parser.parse_args()
    import uuid
    from urllib.parse import urlparse
    review_url = urlparse(os.environ.get("APMM_REVIEW_URL", ""))
    if not args.push and review_url.hostname not in {"127.0.0.1", "localhost", "::1"}:
        raise ValueError("仅本机测试服务允许不推送就回写发布状态")
    if str(uuid.UUID(args.id))!=args.id:raise ValueError("投稿编号无效")
    if args.push and (not args.public_base or not args.public_base.startswith("https://cnb.cool/astral-weird-egg/astral-party-mod-manager-assets/-/git/raw/")):
        raise ValueError("需要固定 CNB 公开来源用于发布复核")
    if git(args.repo,"status","--porcelain"):raise ValueError("发布仓库存在未提交改动，请先检查失败现场")
    args.stage.mkdir(parents=True,exist_ok=True)
    key=Ed25519PrivateKey.from_private_bytes(base64.b64decode(os.environ["APMM_PUBLISH_PRIVATE_KEY"],validate=True))
    allowed=set(json.loads(args.index.read_text(encoding="utf-8-sig")))
    route=f"/v1/admin/submissions/{args.id}"
    with client() as api:
        response=api.post(route+"/begin",json={"operation":args.operation});response.raise_for_status();intent=response.json()
        if intent["done"]:print("发布状态已完成");return
        archive=args.stage/"review.zip"
        if args.operation=="publish":
            with api.stream("GET",route+"/archive") as response,archive.open("wb") as out:
                response.raise_for_status();size=0
                for chunk in response.iter_bytes():
                    size+=len(chunk)
                    if size>80*1024*1024:raise ValueError("待审包超限")
                    out.write(chunk)
        result=build(args.repo/"workshop",args.stage,intent,archive,key,allowed)
        # 资源与两个兼容目录在同一 Git 提交可见；v2 单响应避免签名跨版本读取。
        git(args.repo,"add","--","workshop")
        if git(args.repo,"diff","--cached","--name-only"):
            git(args.repo,"-c","user.name=APMM Workshop","-c","user.email=workshop@invalid.local","commit","-m",f"workshop: {args.operation} {args.id}")
        if args.push:
            git(args.repo,"push","origin","HEAD:main")
            with httpx.Client(timeout=60,follow_redirects=False) as public:
                files=["manifest-v2.json","manifest.json","manifest.sig"]
                work=result["work"]
                if work:files += [work["archive"],*([work["cover"]] if work.get("cover") else []),*work.get("previews",[])]
                for name in files:
                    response=public.get(args.public_base.rstrip("/")+"/"+name);response.raise_for_status()
                    if hashlib.sha256(response.content).digest()!=hashlib.sha256((args.repo/"workshop"/name).read_bytes()).digest():raise ValueError("公开发布文件尚未匹配")
        response=api.post(route+"/finish",json=result);response.raise_for_status()
        print("目录与发布回执已确认")

if __name__=="__main__":
    try:main()
    except Exception:
        print("发布未完成；当前已发布版本保持有效，请保留工作目录并使用同一投稿编号重试。",file=sys.stderr);raise SystemExit(1)
