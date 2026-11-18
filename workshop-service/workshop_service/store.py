"""SQLite 拥有作品、投稿和发布状态；所有状态转换在即时事务中完成。"""
from __future__ import annotations
import contextlib
import hashlib
import hmac
import json
import sqlite3
import time
from pathlib import Path

class Conflict(Exception): pass
class Forbidden(Exception): pass

class Store:
    def __init__(self, root: Path):
        self.root = root
        root.mkdir(parents=True, exist_ok=True)
        self.path = root / "workshop.sqlite3"
        with self.transaction() as db:
            db.executescript("""
                CREATE TABLE IF NOT EXISTS works(id TEXT PRIMARY KEY, credential_hash TEXT NOT NULL,
                    revision INTEGER NOT NULL DEFAULT 0, published TEXT);
                CREATE TABLE IF NOT EXISTS submissions(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id),
                    snapshot_hash TEXT NOT NULL, state TEXT NOT NULL, metadata TEXT NOT NULL,
                    created INTEGER NOT NULL, evidence TEXT, lease TEXT, result TEXT);
                CREATE TABLE IF NOT EXISTS publication(singleton INTEGER PRIMARY KEY CHECK(singleton=1),
                    submission_id TEXT, lease TEXT, operation TEXT);
                INSERT OR IGNORE INTO publication(singleton) VALUES(1);
            """)

    @contextlib.contextmanager
    def transaction(self):
        db = sqlite3.connect(self.path, timeout=10)
        db.row_factory = sqlite3.Row
        db.execute("PRAGMA foreign_keys=ON")
        db.execute("PRAGMA synchronous=FULL")
        try:
            db.execute("BEGIN IMMEDIATE")
            yield db
            db.commit()
        except BaseException:
            db.rollback()
            raise
        finally:
            db.close()

    def authenticate(self, db, id, token):
        row = db.execute("SELECT * FROM works WHERE id=?", (id,)).fetchone()
        if not row or not hmac.compare_digest(row["credential_hash"], hashlib.sha256(token.encode()).hexdigest()):
            raise Forbidden()
        return row

    def register(self, id, token):
        with self.transaction() as db:
            if not db.execute("SELECT 1 FROM works WHERE id=?", (id,)).fetchone():
                if db.execute("SELECT COUNT(*) FROM works").fetchone()[0] >= 10000:
                    raise Conflict("作品配额已满")
                db.execute("INSERT INTO works(id,credential_hash) VALUES(?,?)", (id, hashlib.sha256(token.encode()).hexdigest()))
            self.authenticate(db, id, token)
        return {"id":id}

    @staticmethod
    def receipt(row):
        return {"id": row["id"], "workId":row["work_id"], "snapshotHash":row["snapshot_hash"], "status":row["state"]}

    def own(self, id, token):
        with self.transaction() as db:
            row = self.authenticate(db, id, token)
            entries = db.execute("SELECT * FROM submissions WHERE work_id=? ORDER BY created DESC LIMIT 100", (id,)).fetchall()
            return {"id":id, "revision":row["revision"], "published":json.loads(row["published"]) if row["published"] else None,
                    "submissions":[self.receipt(entry) for entry in entries]}

    def existing(self, work_id, id, token, sha):
        with self.transaction() as db:
            self.authenticate(db, work_id, token)
            row = db.execute("SELECT * FROM submissions WHERE id=?", (id,)).fetchone()
            if row:
                if row["work_id"] != work_id or row["snapshot_hash"] != sha:
                    raise Conflict("幂等编号已经绑定其他内容")
                return self.receipt(row)
        return None

    def receive(self, work_id, id, token, sha, metadata, stage):
        with self.transaction() as db:
            work = self.authenticate(db, work_id, token)
            row = db.execute("SELECT * FROM submissions WHERE id=?", (id,)).fetchone()
            if row:
                if row["work_id"] != work_id or row["snapshot_hash"] != sha:
                    raise Conflict("幂等编号冲突")
                return self.receipt(row)
            if work["revision"] != metadata["baseRevision"] or (metadata["metadataOnly"] and not work["published"]):
                raise Conflict("已发布基线已变化")
            count = db.execute("SELECT COUNT(*) FROM submissions WHERE work_id=?", (work_id,)).fetchone()[0]
            active = db.execute("SELECT COUNT(*) FROM submissions WHERE work_id=? AND state IN ('pending','approved','publishing')", (work_id,)).fetchone()[0]
            if count >= 1000 or active >= 10:
                raise Conflict("投稿配额已满")
            final = self.root / "submissions" / id
            final.parent.mkdir(exist_ok=True)
            if final.exists():
                # 文件提交后数据库提交前断电留下的孤儿只在摘要相同的重试中复用。
                from .inspection import digest
                if digest(final / "upload.zip") != sha:
                    raise Conflict("暂存内容冲突")
            else:
                stage.rename(final)
            db.execute("INSERT INTO submissions(id,work_id,snapshot_hash,state,metadata,created) VALUES(?,?,?,'pending',?,?)",
                       (id,work_id,sha,json.dumps(metadata,ensure_ascii=False),int(time.time())))
            return self.receipt(db.execute("SELECT * FROM submissions WHERE id=?", (id,)).fetchone())

    def cancel(self, work_id, id, token):
        with self.transaction() as db:
            self.authenticate(db,work_id,token)
            row = db.execute("SELECT * FROM submissions WHERE id=? AND work_id=?", (id,work_id)).fetchone()
            if not row or row["state"] not in {"pending","cancelled"}:
                raise Conflict("仅可取消待审投稿")
            db.execute("UPDATE submissions SET state='cancelled' WHERE id=?", (id,))
        return {"id":id,"status":"cancelled"}

    def review(self, id, action, evidence):
        with self.transaction() as db:
            row = db.execute("SELECT * FROM submissions WHERE id=?", (id,)).fetchone()
            if not row or row["state"] != "pending":
                raise Conflict("投稿已不在待审状态")
            if evidence["snapshotHash"] != row["snapshot_hash"]:
                raise Conflict("审核证据与内容摘要不符")
            db.execute("UPDATE submissions SET state=?,evidence=? WHERE id=?",
                       ("approved" if action=="approve" else "rejected",json.dumps(evidence,ensure_ascii=False),id))
        return {"id":id,"status":"approved" if action=="approve" else "rejected"}

    def get_review(self, id):
        with self.transaction() as db:
            row = db.execute("SELECT * FROM submissions WHERE id=?",(id,)).fetchone()
            if not row:
                raise Conflict("投稿不存在")
            return {**self.receipt(row),"submission":json.loads(row["metadata"]),"evidence":json.loads(row["evidence"]) if row["evidence"] else None}
