#requires -Version 7.2
<#
.SYNOPSIS
使用临时本地附件验证 GitHub Release 上传脚本的无网络预检。
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$scriptPath = Join-Path $PSScriptRoot 'Upload-GitHubReleaseAssets.ps1'
$testBase = 'B:\APMM-next-work\release-workflow-tests'
$null = New-Item -ItemType Directory -Path $testBase -Force
$tempRoot = Join-Path $testBase ('apmm-github-release-test-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $tempRoot

try {
    $stage = Join-Path $tempRoot 'previews'
    $null = New-Item -ItemType Directory -Path $stage
    $fixturePath = Join-Path $stage 'preview.zip'
    [IO.File]::WriteAllBytes($fixturePath, [Text.Encoding]::UTF8.GetBytes('local preview fixture'))
    $bytes = [IO.File]::ReadAllBytes($fixturePath)
    $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
    $notesPath = Join-Path $stage 'RELEASE-NOTES.md'
    [IO.File]::WriteAllText($notesPath, 'Local test notes')
    $notesHash = (Get-FileHash -LiteralPath $notesPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $plan = @{
        files = @(
            @{ channel = 'github'; path = 'previews/preview-v4/preview.zip'; bytes = $bytes.Length; sha256 = $hash },
            @{ channel = 'github'; path = 'previews/preview-v4/RELEASE-NOTES.md'; bytes = (Get-Item -LiteralPath $notesPath).Length; sha256 = $notesHash }
        )
        github = @{
            previews = @{
                repository = 'a190141074/AstralPartyModManager-Previews'
                tag = 'preview-v4'
                directory = $stage
                notesFile = 'RELEASE-NOTES.md'
                attachments = @('preview.zip')
            }
        }
    }
    $planPath = Join-Path $tempRoot 'publication-plan.json'
    $plan | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $planPath -Encoding utf8

    $checked = & $scriptPath -Repository Previews -PlanPath $planPath -Check
    if ($checked.Slug -cne 'a190141074/AstralPartyModManager-Previews' -or
        $checked.Tag -cne 'preview-v4' -or $checked.Assets.Count -ne 1 -or
        $checked.Assets[0].Bytes -ne $bytes.Length -or $checked.Assets[0].Sha256 -cne $hash) {
        throw '有效暂存目录未通过摘要预检。'
    }

    $rejected = $false
    [IO.File]::WriteAllText($fixturePath, 'changed local preview fixture')
    try { & $scriptPath -Repository Previews -PlanPath $planPath -Check | Out-Null }
    catch { $rejected = $_.Exception.Message -match '本地附件与本轮计划摘要不符' }
    if (-not $rejected) { throw '附件摘要变化未被拒绝。' }

    [IO.File]::WriteAllBytes($fixturePath, $bytes)
    [IO.File]::WriteAllText((Join-Path $stage 'unplanned.bin'), 'unplanned')
    $rejected = $false
    try { & $scriptPath -Repository Previews -PlanPath $planPath -Check | Out-Null }
    catch { $rejected = $_.Exception.Message -match '未列入发布计划' }
    if (-not $rejected) { throw '计划之外的暂存文件未被拒绝。' }

    Remove-Item -LiteralPath (Join-Path $stage 'unplanned.bin') -Force
    $rejected = $false
    $plan.github.previews.repository = 'attacker.example/not-allowed'
    $plan | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $planPath -Encoding utf8
    try { & $scriptPath -Repository Previews -PlanPath $planPath -Check | Out-Null }
    catch { $rejected = $_.Exception.Message -match '仓库或标签与固定目标' }
    if (-not $rejected) { throw '未授权的 Release 仓库没有被拒绝。' }

    [pscustomobject]@{
        Result = 'Passed'
        Cases = @('Valid staging and digest', 'Changed payload rejected', 'Unplanned file rejected', 'Unapproved repository rejected')
        Network = 'Not used'
        Credentials = 'Not read'
    }
} finally {
    if (Test-Path -LiteralPath $tempRoot -PathType Container) {
        $resolvedRoot = (Resolve-Path -LiteralPath $tempRoot).ProviderPath
        if ([IO.Path]::GetDirectoryName($resolvedRoot) -ieq $testBase.TrimEnd([IO.Path]::DirectorySeparatorChar) -and
            [IO.Path]::GetFileName($resolvedRoot) -match '\Aapmm-github-release-test-[0-9a-f]{32}\z') {
            Remove-Item -LiteralPath $resolvedRoot -Recurse -Force
        }
    }
}
