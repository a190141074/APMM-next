"""用途：控制取消、进程优先级和资源统计。作者：待填写。修改记录：2026-11-14，首次创建。"""

import ctypes
import os
import signal
import threading
import time

from .errors import Cancelled


class Runtime:
    """保存取消信号与本次运行的资源统计。"""

    def __init__(self):
        """功能：创建运行上下文。入参：无。返回：无。异常：无。"""
        self.cancelled = threading.Event()
        self.started, self.cpu_started = time.perf_counter(), time.process_time()
        self.peak_bytes = 0
        self.warnings = []

    def install(self) -> None:
        """功能：启用温和优先级和取消处理。入参：无。返回：无。异常：无，失败记录提示。"""
        signal.signal(signal.SIGINT, self._signal)
        if hasattr(signal, "SIGTERM"):
            signal.signal(signal.SIGTERM, self._signal)
        if os.name == "nt":
            kernel = ctypes.WinDLL("kernel32", use_last_error=True)
            kernel.GetCurrentProcess.restype = ctypes.c_void_p
            kernel.SetPriorityClass.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
            if not kernel.SetPriorityClass(kernel.GetCurrentProcess(), 0x00004000):
                self.warnings.append("无法设置低进程优先级，仍按单线程逐包处理。")

    def _signal(self, number, frame) -> None:
        """功能：记录取消请求。入参：信号编号和当前栈帧。返回：无。异常：无。"""
        self.cancelled.set()

    def checkpoint(self) -> None:
        """功能：检查取消并采样内存。入参：无。返回：无。异常：已取消。"""
        if self.cancelled.is_set():
            raise Cancelled("已取消操作，尚未完成的提交已回滚。")
        self.peak_bytes = max(self.peak_bytes, peak_memory())

    def pause(self, milliseconds: int) -> None:
        """功能：让出处理时间并响应取消。入参：间隔毫秒数。返回：无。异常：已取消。"""
        self.cancelled.wait(milliseconds / 1000)
        self.checkpoint()

    def metrics(self) -> dict:
        """功能：取得本次资源统计。入参：无。返回：耗时、处理器时间和峰值内存。异常：无。"""
        return {"duration_seconds": round(time.perf_counter() - self.started, 3),
                "cpu_seconds": round(time.process_time() - self.cpu_started, 3),
                "peak_memory_mib": round(max(self.peak_bytes, peak_memory()) / (1024 * 1024), 2)}


def peak_memory() -> int:
    """功能：读取本进程峰值工作集。入参：无。返回：字节数，不可用时为零。异常：无。"""
    if os.name != "nt":
        try:
            import resource
            return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024
        except ImportError:
            return 0
    class MemoryCounters(ctypes.Structure):
        """声明 Windows 进程内存统计结构。"""
        _fields_ = [("cb", ctypes.c_uint32), ("PageFaultCount", ctypes.c_uint32),
                    ("PeakWorkingSetSize", ctypes.c_size_t), ("WorkingSetSize", ctypes.c_size_t),
                    ("QuotaPeakPagedPoolUsage", ctypes.c_size_t), ("QuotaPagedPoolUsage", ctypes.c_size_t),
                    ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t), ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
                    ("PagefileUsage", ctypes.c_size_t), ("PeakPagefileUsage", ctypes.c_size_t)]
    kernel, psapi = ctypes.WinDLL("kernel32"), ctypes.WinDLL("psapi")
    kernel.GetCurrentProcess.restype = ctypes.c_void_p
    psapi.GetProcessMemoryInfo.argtypes = [ctypes.c_void_p, ctypes.POINTER(MemoryCounters), ctypes.c_uint32]
    counters = MemoryCounters()
    counters.cb = ctypes.sizeof(counters)
    if not psapi.GetProcessMemoryInfo(kernel.GetCurrentProcess(), ctypes.byref(counters), counters.cb):
        return 0
    return counters.PeakWorkingSetSize

