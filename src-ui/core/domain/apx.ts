// APX 的文件状态使用校验结果，不能把文件存在等同于补丁已安装。
export function apxFileState(file: {
  present: boolean;
  installed: boolean;
  validationError?: string | null;
}) {
  if (file.installed) return "已安装";
  if (!file.present) return "缺失";
  if (file.validationError) return "校验失败";
  return "待更新";
}
export function apxInstallState(data: {
  totalFiles?: number;
  installedFiles?: number;
  localPluginVersion?: string;
  remoteVersion?: string;
  files?: {
    present: boolean;
    installed: boolean;
    validationError?: string | null;
  }[];
}) {
  if (
    data.localPluginVersion &&
    data.remoteVersion &&
    data.localPluginVersion !== data.remoteVersion
  )
    return "待更新";
  if (data.totalFiles && data.installedFiles === data.totalFiles)
    return "已安装";
  if (data.files?.some((f) => f.validationError)) return "校验失败";
  if (data.installedFiles) return "部分安装";
  return data.totalFiles ? "未安装" : "待检查";
}
