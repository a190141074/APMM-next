"""投稿格式和资源边界；拒绝未声明字段及未分类资源。"""
from __future__ import annotations
import re
from urllib.parse import urlsplit
from pydantic import BaseModel, ConfigDict, Field, field_validator

MAX_UPLOAD = 80 * 1024 * 1024
MAX_ARCHIVE = 64 * 1024 * 1024
MAX_EXPANDED = 256 * 1024 * 1024
MAX_IMAGE = 2 * 1024 * 1024

class StrictModel(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)

def source_name(value: str) -> str:
    stem = value.split(".")[0].upper()
    if (not value or len(value.encode()) > 240 or not value.lower().endswith(".png")
        or value.endswith((" ", ".")) or any(ord(c) < 32 or c in '<>:"/\\|?*' for c in value)
        or stem in {"CON", "PRN", "AUX", "NUL"} or re.fullmatch(r"(?:COM|LPT)[0-9]", stem)):
        raise ValueError("无效的资源展示名")
    return value

class Metadata(StrictModel):
    title: str = Field(min_length=1, max_length=120)
    author: str = Field(default="", max_length=120)
    artist: str = Field(default="", max_length=120)
    authorUrls: list[str] = Field(default_factory=list, max_length=8)
    artistUrls: list[str] = Field(default_factory=list, max_length=8)
    sourceUrl: str = ""
    description: str = Field(min_length=1, max_length=10000)
    category: str = Field(default="", max_length=64)
    tags: list[str] = Field(default_factory=list, max_length=16)
    version: str = Field(min_length=1, max_length=64)
    gameVersion: str = Field(default="", max_length=256)
    changeNote: str = Field(default="", max_length=16000)

    @field_validator("authorUrls", "artistUrls", "sourceUrl")
    @classmethod
    def public_links(cls, value):
        links = [value] if isinstance(value, str) and value else value
        for link in links:
            url = urlsplit(link)
            if (not link or link != link.strip() or len(link.encode()) > 2048
                    or url.scheme != "https" or not url.hostname or url.username or url.password
                    or any(ord(c) < 32 for c in link)):
                raise ValueError("链接须为不含登录凭据的 HTTPS 网址，且最多 2048 字节")
        return value

    @field_validator("title", "description", "version")
    @classmethod
    def nonempty(cls, value):
        if not value.strip():
            raise ValueError("必填资料不能为空")
        return value

    @field_validator("tags")
    @classmethod
    def tags_valid(cls, value):
        if any(not t.strip() or len(t) > 24 for t in value) or len(set(t.casefold() for t in value)) != len(value):
            raise ValueError("标签重复或长度无效")
        return value

class File(StrictModel):
    name: str = Field(pattern=r"^[a-zA-Z0-9_-]{1,100}\.png$")
    sourceName: str
    canonical: str = Field(max_length=180)
    sha256: str = Field(pattern=r"^[0-9a-f]{64}$")
    bytes: int = Field(gt=0, le=MAX_EXPANDED)
    _source = field_validator("sourceName")(source_name)

class Submission(StrictModel):
    model_config = ConfigDict(extra="forbid", strict=True, serialize_by_alias=True)
    schema_version: int = Field(alias="schema", ge=1, le=1)
    workId: str | None = Field(default=None, pattern=r"^[0-9a-f-]{36}$")
    submissionId: str | None = Field(default=None, pattern=r"^[0-9a-f-]{36}$")
    metadata: Metadata
    metadataOnly: bool
    baseRevision: int = Field(ge=0)
    files: list[File] = Field(max_length=512)
    images: list[File] = Field(max_length=7)

class Evidence(StrictModel):
    snapshotHash: str = Field(pattern=r"^[0-9a-f]{64}$")
    scanResult: str = Field(min_length=1, max_length=2000)
    gameVersion: str = Field(min_length=1, max_length=256)
    testEvidence: str = Field(min_length=1, max_length=4000)
    reviewNote: str = Field(min_length=1, max_length=4000)

    @field_validator("scanResult", "gameVersion", "testEvidence", "reviewNote")
    @classmethod
    def evidence_present(cls, value):
        if not value.strip():
            raise ValueError("审核证据不能为空")
        return value
