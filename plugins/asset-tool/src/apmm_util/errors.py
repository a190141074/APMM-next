"""用途：定义可报告异常与稳定错误信息。作者：待填写。修改记录：2026-11-14，补充错误码及定位上下文。"""


class ToolError(Exception):
    """表示配置、目录或资产处理失败，并保存机器可读的原因。"""

    def __init__(self, message: str, *, code="processing_error", stage=None, context=None):
        """功能：创建可报告异常。入参：消息、错误码、阶段和上下文。返回：异常实例。异常：无。"""
        super().__init__(message)
        self.code, self.stage, self.context = code, stage, dict(context or {})


class CatalogChanged(ToolError):
    """表示游戏在本次操作期间修改了目录或已定位资源。"""


class Cancelled(ToolError):
    """表示操作已取消，尚未提交的结果应当回滚。"""


class ConfirmationRequired(ToolError):
    """表示非交互调用需要明确提供更新授权。"""


def error_details(exc: Exception, *, stage="command", context=None) -> dict:
    """功能：转换稳定错误报告。入参：异常、默认阶段及上下文。返回：消息、错误码、阶段和上下文。异常：无。"""
    defaults = ((ConfirmationRequired, "confirmation_required"), (CatalogChanged, "catalog_changed"),
                (Cancelled, "cancelled"), (KeyboardInterrupt, "cancelled"), (FileNotFoundError, "bundle_missing"),
                (OSError, "io_error"), (ImportError, "dependency_missing"), (ValueError, "invalid_data"))
    code = getattr(exc, "code", "processing_error")
    if code == "processing_error":
        code = next((value for kind, value in defaults if isinstance(exc, kind)), code)
    details = dict(context or {})
    details.update(getattr(exc, "context", {}))
    return {"message": str(exc) or "操作已取消。", "code": code,
            "stage": getattr(exc, "stage", None) or stage, "context": details}
