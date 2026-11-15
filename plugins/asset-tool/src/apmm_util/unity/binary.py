"""用途：提供有边界校验的二进制读取。作者：待填写。修改记录：2026-11-14，首次创建。"""

import struct

from ..errors import ToolError


class Reader:
    """封装二进制位置，防止损坏数据越界读取。"""

    def __init__(self, data: bytes, endian: str = ">"):
        """功能：创建读取器。入参：字节数据和字节序。返回：无。异常：无。"""
        self.data, self.endian, self.position = data, endian, 0

    def take(self, size: int) -> bytes:
        """功能：读取指定长度。入参：字节数。返回：字节片段。异常：长度无效或越界。"""
        end = self.position + size
        if size < 0 or end > len(self.data):
            raise ToolError("二进制数据截断或长度无效。")
        value = self.data[self.position:end]
        self.position = end
        return value

    def number(self, code: str) -> int:
        """功能：读取整数。入参：结构格式。返回：整数。异常：截断或格式无效。"""
        return struct.unpack(self.endian + code, self.take(struct.calcsize(code)))[0]

    def cstring(self, limit: int = 4096) -> str:
        """功能：读取零结尾字符串。入参：长度上限。返回：字符串。异常：截断或编码无效。"""
        end = self.data.find(b"\0", self.position, self.position + limit + 1)
        if end < 0:
            raise ToolError("二进制字符串未结束或超过上限。")
        value = self.take(end - self.position).decode("utf-8")
        self.take(1)
        return value

    def align(self, size: int = 16) -> None:
        """功能：移动至对齐位置。入参：对齐字节数。返回：无。异常：对齐越界。"""
        target = (self.position + size - 1) // size * size
        self.take(target - self.position)

