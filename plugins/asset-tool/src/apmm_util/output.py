"""用途：锁定输出并事务提交图片和状态。作者：待填写。修改记录：2026-11-14，首次创建。"""

import json
import os
import shutil
import tempfile
from pathlib import Path
from typing import Callable

from .errors import ToolError


class OutputLock:
    """以系统文件锁阻止多个同步程序同时写入。"""

    def __init__(self, root: Path):
        """功能：创建锁描述。入参：输出根目录。返回：无。异常：无。"""
        self.root, self.stream = root, None

    def __enter__(self):
        """功能：取得独占文件锁。入参：无。返回：锁对象。异常：输出被占用。"""
        self.root.mkdir(parents=True, exist_ok=True)
        self.stream = (self.root / ".apmm-util.lock").open("a+b")
        self.stream.seek(0, os.SEEK_END)
        if self.stream.tell() == 0:
            self.stream.write(b"\0")
            self.stream.flush()
        self.stream.seek(0)
        try:
            if os.name == "nt":
                import msvcrt
                msvcrt.locking(self.stream.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(self.stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as exc:
            self.stream.close()
            raise ToolError("另一个 apmm-util 正在写入该输出目录，请等待其完成。") from exc
        return self

    def __exit__(self, kind, value, traceback) -> None:
        """功能：释放文件锁。入参：上下文异常信息。返回：无。异常：无。"""
        if self.stream:
            self.stream.close()


class Transaction:
    """暂存全部输出，提交失败时恢复旧文件。"""

    def __init__(self, root: Path):
        """功能：创建独立暂存目录。入参：输出根目录。返回：无。异常：创建失败。"""
        self.root = root.resolve()
        self.root.mkdir(parents=True, exist_ok=True)
        work = os.environ.get('APMM_WORK_DIR') or ('B:/APMM-next-work' if os.name == 'nt' and Path('B:/').is_dir() else str(Path(tempfile.gettempdir()) / 'apmm-next-work'))
        self.staging_root = (Path(work) / 'staging').resolve()
        self.staging_root.mkdir(parents=True, exist_ok=True)
        self.stage = Path(tempfile.mkdtemp(prefix="asset-", dir=self.staging_root)).resolve()
        self.pending = {}
        self.preserve = False

    def path(self, relative: str) -> Path:
        """功能：取得安全暂存路径。入参：输出相对路径。返回：暂存文件路径。异常：路径越界。"""
        rel = Path(relative)
        if rel.is_absolute() or ".." in rel.parts:
            raise ToolError("输出路径无效。")
        target = (self.root / rel).resolve()
        if not target.is_relative_to(self.root) or target == self.root:
            raise ToolError("输出路径越出了工具数据目录。")
        staged = self.stage / "new" / rel
        staged.parent.mkdir(parents=True, exist_ok=True)
        self.pending[relative] = (staged, target)
        return staged

    def json(self, relative: str, document: dict | list) -> None:
        """功能：暂存结构化输出。入参：相对路径和文档。返回：无。异常：序列化或写入失败。"""
        data = json.dumps(document, ensure_ascii=False, indent=2).encode("utf-8") + b"\n"
        with self.path(relative).open("wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())

    def discard(self, relative: str) -> None:
        """功能：丢弃失败条目的暂存产物。入参：相对路径。返回：无。异常：文件清理失败。"""
        pending = self.pending.pop(relative, None)
        if pending:
            pending[0].unlink(missing_ok=True)

    def png(self, relative: str, image) -> Path:
        """功能：低压缩开销暂存原图。入参：相对路径和图像。返回：暂存路径。异常：编码或写入失败。"""
        target = self.path(relative)
        try:
            with target.open("wb") as stream:
                image.save(stream, format="PNG", compress_level=3)
                stream.flush()
                os.fsync(stream.fileno())
        except Exception:
            self.pending.pop(relative, None)
            target.unlink(missing_ok=True)
            raise
        return target

    def commit(self, checkpoint: Callable[[], None]) -> None:
        """功能：提交暂存结果并在失败时回滚。入参：一致性和取消检查函数。返回：无。异常：检查或写入失败。"""
        applied = []
        try:
            for number, (_, (staged, target)) in enumerate(self.pending.items()):
                checkpoint()
                target.parent.mkdir(parents=True, exist_ok=True)
                backup = self.stage / "old" / str(number)
                if target.exists():
                    backup.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(target, backup)
                else:
                    backup = None
                # 暂存根可能与输出跨卷，先复制到目标旁再原子替换。
                adjacent = target.with_name(target.name + '.' + self.stage.name + '.pending')
                try:
                    shutil.copyfile(staged, adjacent)
                    os.replace(adjacent, target)
                finally:
                    adjacent.unlink(missing_ok=True)
                applied.append((target, backup))
            checkpoint()
        except BaseException:
            failures = []
            for target, backup in reversed(applied):
                try:
                    if backup:
                        shutil.copyfile(backup, target)
                    else:
                        target.unlink(missing_ok=True)
                except OSError as exc:
                    failures.append(str(exc))
            if failures:
                self.preserve = True
                raise ToolError(f"提交失败且部分文件恢复失败，备份保留在 {self.stage}：{'；'.join(failures)}")
            raise

    def cleanup(self) -> None:
        """功能：删除本次独占暂存目录。入参：无。返回：无。异常：路径校验或清理失败。"""
        if self.preserve:
            return
        resolved = self.stage.resolve()
        if resolved.parent != self.staging_root or not resolved.name.startswith("asset-") or self.stage.is_symlink():
            raise ToolError("暂存目录校验失败，停止清理。")
        if resolved.exists():
            shutil.rmtree(resolved)

