"""维护者读取、批准、拒绝和生成脱敏 Issue 资料；不会执行上传内容。"""
import argparse
import hashlib
import json
import os
import sys
from pathlib import Path
import httpx

def client():
    return httpx.Client(base_url=os.environ["APMM_REVIEW_URL"].rstrip("/"),
                        headers={"Authorization":"Bearer "+os.environ["APMM_REVIEW_TOKEN"]},follow_redirects=False,timeout=120)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action",choices=["inspect","fetch","issue","approve","reject"])
    parser.add_argument("id",type=str)
    parser.add_argument("--evidence",type=Path)
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()
    import uuid
    if str(uuid.UUID(args.id))!=args.id: raise ValueError("投稿编号无效")
    route=f"/v1/admin/submissions/{args.id}"
    with client() as api:
        report=api.get(route);report.raise_for_status();report=report.json()
        if args.action=="fetch":
            if not args.output: raise ValueError("需要私有输出路径")
            sha=hashlib.sha256();size=0
            with api.stream("GET",route+"/archive") as response,args.output.open("xb") as output:
                response.raise_for_status()
                for chunk in response.iter_bytes():
                    size+=len(chunk)
                    if size>80*1024*1024: raise ValueError("待审包超限")
                    sha.update(chunk);output.write(chunk)
            if sha.hexdigest()!=report["snapshotHash"]: raise ValueError("待审包摘要不符")
            print("待审包已保存并校验")
        elif args.action in {"approve","reject"}:
            if not args.evidence: raise ValueError("需要审核证据文件")
            response=api.post(route+"/"+args.action,json=json.loads(args.evidence.read_text(encoding="utf-8-sig")))
            response.raise_for_status();print(json.dumps(response.json(),ensure_ascii=False))
        elif args.action=="issue":
            if not args.output: raise ValueError("需要 Issue 草稿输出位置")
            safe={"submissionId":report["id"],"snapshotHash":report["snapshotHash"],"metadata":report["submission"]["metadata"],
                  "fileCount":len(report["submission"]["files"]),"expandedBytes":sum(f["bytes"] for f in report["submission"]["files"])}
            # JSON 缩进防止投稿文本成为 Markdown 指令或提及通知，不包含私有附件地址。
            text="APMM 工坊投稿审核\n\n"+"\n".join("    "+line for line in json.dumps(safe,ensure_ascii=False,indent=2).replace("@","＠").splitlines())+"\n"
            args.output.write_text(text,encoding="utf-8")
            print("脱敏审核 Issue 草稿已生成")
        else:
            print(json.dumps(report,ensure_ascii=False,indent=2))

if __name__=="__main__":
    try: main()
    except Exception:
        # HTTP 异常可能携带地址或授权上下文，控制台只输出静态失败说明。
        print("审核操作失败，请检查服务、权限及证据文件。",file=sys.stderr);raise SystemExit(1)
