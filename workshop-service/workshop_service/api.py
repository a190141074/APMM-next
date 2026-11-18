"""HTTPS 反向代理后的单实例收件入口；上传流式限额，维护者接口独立鉴权。"""
from __future__ import annotations
import asyncio
import hashlib
import hmac
import json
import os
import re
import shutil
import time
import uuid
import zipfile
from pathlib import Path
from fastapi import FastAPI, Request, HTTPException
from fastapi.responses import JSONResponse, FileResponse
from starlette.requests import ClientDisconnect
from pydantic import ValidationError
from .models import MAX_UPLOAD, MAX_ARCHIVE, MAX_EXPANDED, Evidence
from .inspection import inspect, unique_json
from .store import Store, Conflict, Forbidden
from . import publication

def identifier(value):
    try:
        if str(uuid.UUID(value)) != value:
            raise ValueError()
    except ValueError:
        raise HTTPException(422, "编号无效") from None
    return value

def bearer(request):
    value = request.headers.get("authorization", "")
    if not value.startswith("Bearer ") or not re.fullmatch(r"[A-Za-z0-9_-]{43}", value[7:]):
        raise HTTPException(401, "需要管理凭证")
    return value[7:]

async def bounded_json(request, limit=64*1024):
    body = bytearray()
    async for data in request.stream():
        body.extend(data)
        if len(body) > limit:
            raise HTTPException(413,"请求过大")
    return unique_json(bytes(body))

def create_app(root: Path, admin_token: str, canonical_index: Path, quota=10*1024**3):
    if not root.is_absolute() or not re.fullmatch(r"[A-Za-z0-9_-]{43}",admin_token):
        raise ValueError("必须配置绝对数据目录与独立维护者凭证")
    store = Store(root)
    allowed = set(json.loads(canonical_index.read_text(encoding="utf-8-sig")))
    incoming = root / "incoming"
    incoming.mkdir(exist_ok=True)
    app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
    app.state.store = store
    lanes = asyncio.Semaphore(2)
    rates = {}

    @app.middleware("http")
    async def limits(request, call_next):
        now = time.monotonic()
        if len(rates) > 4096:
            for key in list(rates):
                if rates[key][0] < now-60:
                    rates.pop(key)
        key = request.client.host if request.client else "unknown"
        start,count = rates.get(key,(now,0))
        if now-start > 60:
            start,count=now,0
        if count >= 120 or len(rates) >= 4096 and key not in rates:
            return JSONResponse({"detail":"请求过于频繁"},429,headers={"Retry-After":"60"})
        rates[key]=(start,count+1)
        return await call_next(request)

    @app.exception_handler(Conflict)
    async def conflict(_request, _error): return JSONResponse({"detail":"状态冲突，请同步后重试"},409)
    @app.exception_handler(Forbidden)
    async def forbidden(_request, _error): return JSONResponse({"detail":"无权操作该作品"},403)
    @app.exception_handler(ValueError)
    @app.exception_handler(ValidationError)
    async def invalid(_request, _error): return JSONResponse({"detail":"请求资料、归档或图片无效"},422)

    def admin(request):
        if not hmac.compare_digest(bearer(request),admin_token):
            raise HTTPException(403,"需要维护者权限")

    @app.get("/v1/health")
    async def health(): return {"schema":1,"maxUpload":MAX_UPLOAD,"maxArchive":MAX_ARCHIVE,"maxExpanded":MAX_EXPANDED,"maxFiles":512,"maxImages":7}

    @app.put("/v1/works/{work_id}")
    async def register(work_id: str, request: Request): return store.register(identifier(work_id),bearer(request))

    @app.get("/v1/works/{work_id}")
    async def own(work_id: str, request: Request): return store.own(identifier(work_id),bearer(request))

    @app.put("/v1/works/{work_id}/submissions/{id}")
    async def receive(work_id: str, id: str, request: Request):
        identifier(work_id); identifier(id)
        token = bearer(request)
        sha = request.headers.get("x-content-sha256", "")
        if not re.fullmatch(r"[0-9a-f]{64}", sha):
            raise HTTPException(422,"需要投稿摘要")
        existing = store.existing(work_id,id,token,sha)
        if existing:
            return existing
        try:
            await asyncio.wait_for(lanes.acquire(),timeout=0.05)
        except TimeoutError:
            raise HTTPException(429,"上传通道繁忙") from None
        stage = incoming / str(uuid.uuid4())
        stage.mkdir()
        try:
            used = await asyncio.to_thread(lambda: sum(p.stat().st_size for p in root.rglob("*") if p.is_file()))
            # 为两个活动请求预留完整接收、展开和内容归档空间。
            if used + 2*(MAX_UPLOAD+MAX_EXPANDED+MAX_ARCHIVE+14*1024**2) > quota:
                raise HTTPException(413,"私有存储配额已满")
            total=0; hash=hashlib.sha256()
            async with asyncio.timeout(120):
                with (stage/"upload.zip").open("xb") as output:
                    async for data in request.stream():
                        total += len(data)
                        if total > MAX_UPLOAD:
                            raise HTTPException(413,"投稿包过大")
                        hash.update(data); output.write(data)
                    output.flush(); os.fsync(output.fileno())
            length = request.headers.get("content-length")
            if hash.hexdigest()!=sha or length is not None and total!=int(length):
                raise HTTPException(422,"投稿包大小或摘要不符")
            try:
                submission = await asyncio.to_thread(inspect,stage/"upload.zip",stage,allowed)
            except (zipfile.BadZipFile, KeyError, ValueError, OSError):
                raise HTTPException(422,"投稿资料、归档或图片无效") from None
            return store.receive(work_id,id,token,sha,submission.model_dump(),stage)
        except (TimeoutError, OSError, ClientDisconnect):
            raise HTTPException(503,"接收中断，可用同一投稿编号重试") from None
        finally:
            # 只清理本次 UUID 暂存，成功目录已由数据库事务移入私有区。
            if stage.exists():
                shutil.rmtree(stage)
            lanes.release()

    @app.delete("/v1/works/{work_id}/submissions/{id}")
    async def cancel(work_id: str, id: str, request: Request): return store.cancel(identifier(work_id),identifier(id),bearer(request))

    @app.get("/v1/admin/submissions/{id}")
    async def review(id: str, request: Request):
        admin(request); return store.get_review(identifier(id))

    @app.get("/v1/admin/submissions/{id}/archive")
    async def archive(id: str, request: Request):
        admin(request); store.get_review(identifier(id))
        return FileResponse(root/"submissions"/id/"upload.zip",media_type="application/zip",filename=f"{id}.zip")

    @app.post("/v1/admin/submissions/{id}/{action}")
    async def transition(id: str, action: str, request: Request):
        admin(request); identifier(id)
        body = await bounded_json(request)
        if action in {"approve","reject"}:
            evidence = Evidence.model_validate(body)
            return store.review(id,action,evidence.model_dump())
        if action == "begin": return publication.begin(store,id,body.get("operation"))
        if action == "finish": return publication.finish(store,id,body)
        raise HTTPException(404,"操作不存在")

    return app

def from_env():
    return create_app(Path(os.environ["APMM_SUBMISSION_DATA"]), os.environ["APMM_REVIEW_TOKEN"],
                      Path(os.environ["APMM_CANONICAL_INDEX"]),int(os.environ.get("APMM_SUBMISSION_QUOTA",10*1024**3)))
