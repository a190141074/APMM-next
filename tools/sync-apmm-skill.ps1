#requires -Version 7.2
<#
.SYNOPSIS
将项目维护的 APMM skill 安装到 Codex，或只读校验安装副本。
.DESCRIPTION
只新增或更新来源文件，不删除安装目录的额外文件。遇到符号链接或目录冲突时停止，避免写出目标范围。
#>
[CmdletBinding()]
param(
    [switch]$Check,
    [string]$TargetRoot = $(if ($env:CODEX_HOME) { Join-Path $env:CODEX_HOME 'skills' } else { Join-Path $env:USERPROFILE '.codex/skills' })
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$skillName = 'apmm-next-modding'
$sourceRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../skill/$skillName"))
if (-not [IO.Path]::IsPathFullyQualified($TargetRoot)) { throw 'TargetRoot 必须是绝对路径。' }
$targetPath = [IO.Path]::GetFullPath((Join-Path $TargetRoot $skillName))
if ($sourceRoot -ieq $targetPath -or $targetPath.StartsWith($sourceRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or $sourceRoot.StartsWith($targetPath + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw '维护源与安装目标必须位于不同且互不包含的目录。'
}
if (-not (Test-Path -LiteralPath (Join-Path $sourceRoot 'SKILL.md') -PathType Leaf)) { throw '项目 skill 维护源缺少 SKILL.md。' }

# 每个现存祖先都必须是普通目录，防止安装目标经链接重定向到其他位置。
function Assert-DirectoryPath {
    param([string]$Path)
    $candidate = $Path
    while ($candidate) {
        if (Test-Path -LiteralPath $candidate) {
            $item = Get-Item -LiteralPath $candidate -Force
            if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "路径必须是普通目录：$candidate" }
        }
        $candidate = [IO.Path]::GetDirectoryName($candidate)
    }
}
Assert-DirectoryPath $sourceRoot
Assert-DirectoryPath $targetPath
$sourceItems = @(Get-ChildItem -LiteralPath $sourceRoot -Recurse -Force)
if ($sourceItems | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }) { throw '维护源不能包含符号链接或目录联接。' }
$targetItems = if (Test-Path -LiteralPath $targetPath) { @(Get-ChildItem -LiteralPath $targetPath -Recurse -Force) } else { @() }
if ($targetItems | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }) { throw '安装目标不能包含符号链接或目录联接。' }

$sourceFiles = @($sourceItems | Where-Object { -not $_.PSIsContainer })
$sourceNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$changes = [Collections.Generic.List[object]]::new()
foreach ($file in $sourceFiles) {
    $relative = [IO.Path]::GetRelativePath($sourceRoot, $file.FullName)
    $null = $sourceNames.Add($relative)
    $destination = Join-Path $targetPath $relative
    Assert-DirectoryPath ([IO.Path]::GetDirectoryName($destination))
    if (Test-Path -LiteralPath $destination -PathType Container) { throw "文件路径已被目录占用：$relative" }
    $sourceHash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
    $state = if (-not (Test-Path -LiteralPath $destination -PathType Leaf)) { 'Missing' } elseif ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -ne $sourceHash) { 'Different' } else { 'Match' }
    if ($state -ne 'Match') { $changes.Add([pscustomobject]@{ File = $relative; Source = $file.FullName; Target = $destination; Hash = $sourceHash; State = $state }) }
}
$extras = @($targetItems | Where-Object { -not $_.PSIsContainer -and -not $sourceNames.Contains([IO.Path]::GetRelativePath($targetPath, $_.FullName)) })

if ($Check) {
    foreach ($change in $changes) { Write-Output ("{0}: {1}" -f $change.State, $change.File) }
} else {
    foreach ($change in $changes) {
        $null = New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($change.Target)) -Force
        Copy-Item -LiteralPath $change.Source -Destination $change.Target -Force
        if ((Get-FileHash -LiteralPath $change.Target -Algorithm SHA256).Hash -ne $change.Hash) { throw "同步后摘要不一致：$($change.File)" }
        Write-Output ("Updated: {0}" -f $change.File)
    }
}
foreach ($extra in $extras) { Write-Warning ("保留安装目标的额外文件：{0}" -f [IO.Path]::GetRelativePath($targetPath, $extra.FullName)) }
if ($Check -and ($changes.Count -gt 0 -or $extras.Count -gt 0)) {
    Write-Output ("Skill check failed: {0} differences, {1} extra files." -f $changes.Count, $extras.Count)
    exit 1
}
if ($extras.Count -eq 0) { Write-Output ("Skill mirrors match: {0} files. Target: {1}" -f $sourceFiles.Count, $targetPath) }
else { Write-Output ("Source files synchronized; {0} extra files preserved." -f $extras.Count) }
