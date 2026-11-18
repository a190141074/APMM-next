"""发布交接状态；单一持久租约使中断重试不与另一次目录写入交错。"""
import json
import uuid
from .store import Conflict

def begin(store, id, operation):
    if operation not in {"publish", "withdraw"}:
        raise Conflict("发布操作无效")
    with store.transaction() as db:
        row = db.execute("SELECT * FROM submissions WHERE id=?", (id,)).fetchone()
        if not row:
            raise Conflict("投稿不存在")
        work = db.execute("SELECT * FROM works WHERE id=?", (row["work_id"],)).fetchone()
        current = json.loads(work["published"]) if work["published"] else None
        done = (operation == "publish" and row["state"] == "published") or (operation == "withdraw" and row["state"] == "withdrawn")
        if done:
            return {"done":True,"result":json.loads(row["result"]) if row["result"] else None}
        active = db.execute("SELECT * FROM publication WHERE singleton=1").fetchone()
        if active["submission_id"]:
            if active["submission_id"] != id or active["operation"] != operation:
                raise Conflict("另一个发布事务尚未完成")
            lease = active["lease"]
        else:
            meta = json.loads(row["metadata"])
            if operation == "publish":
                if row["state"] != "approved" or work["revision"] != meta["baseRevision"]:
                    raise Conflict("投稿未获批准或基线已变化")
            elif row["state"] != "published" or not current or current.get("submissionId") != id:
                raise Conflict("只能撤回当前已发布的投稿")
            lease = str(uuid.uuid4())
            db.execute("UPDATE publication SET submission_id=?,lease=?,operation=? WHERE singleton=1", (id,lease,operation))
            if operation == "publish":
                db.execute("UPDATE submissions SET state='publishing',lease=? WHERE id=?", (lease,id))
        return {"done":False,"lease":lease,"operation":operation,"id":id,"workId":row["work_id"],
                "revision":work["revision"]+1,"published":current,
                "snapshotHash":row["snapshot_hash"],"submission":json.loads(row["metadata"])}

def finish(store, id, body):
    with store.transaction() as db:
        row = db.execute("SELECT * FROM submissions WHERE id=?",(id,)).fetchone()
        if not row:
            raise Conflict("投稿不存在")
        if row["result"] and json.loads(row["result"]) == body:
            return {"id":id,"status":row["state"]}
        active = db.execute("SELECT * FROM publication WHERE singleton=1").fetchone()
        if active["submission_id"] != id or active["lease"] != body.get("lease") or active["operation"] != body.get("operation"):
            raise Conflict("发布租约不匹配")
        if not isinstance(body.get("catalogHash"), str) or len(body["catalogHash"]) != 64:
            raise Conflict("缺少发布目录摘要")
        operation = active["operation"]
        if operation == "publish":
            work = body.get("work", {})
            previous = db.execute("SELECT * FROM works WHERE id=?",(row["work_id"],)).fetchone()
            if work.get("id") != row["work_id"] or work.get("submissionId") != id or work.get("revision") != previous["revision"]+1:
                raise Conflict("发布回执与作品修订不匹配")
            db.execute("UPDATE works SET revision=revision+1,published=? WHERE id=?",(json.dumps(work,ensure_ascii=False),row["work_id"]))
            state = "published"
        else:
            # 保留作者的已发布基线供后续更新，公开目录由发布器移除。
            db.execute("UPDATE works SET revision=revision+1 WHERE id=?",(row["work_id"],))
            state = "withdrawn"
        db.execute("UPDATE submissions SET state=?,result=? WHERE id=?",(state,json.dumps(body,ensure_ascii=False),id))
        db.execute("UPDATE publication SET submission_id=NULL,lease=NULL,operation=NULL WHERE singleton=1")
        return {"id":id,"status":state}
