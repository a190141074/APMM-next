#requires -Version 7.2
<#
.SYNOPSIS
按本轮发布计划安全续传 GitHub Release 草稿附件。
.DESCRIPTION
默认只在本机核对计划和附件的文件名、大小与 SHA-256。只有 -Execute 会读取令牌、访问 GitHub 并上传。
仓库、标签和附件清单来自 publication-plan.json，远端只允许 APMM 应用和原图预览两个仓库。
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet('App', 'Previews')]
    [string]$Repository,

    [Parameter(Mandatory)]
    [string]$PlanPath,

    [switch]$Check,

    [switch]$Execute
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:RepositoryTargets = @{
    App = @{ Key = 'application'; Slug = 'a190141074/APMM-next'; TagPattern = '\Av[0-9]+\.[0-9]+\.[0-9]+\z' }
    Previews = @{ Key = 'previews'; Slug = 'a190141074/AstralPartyModManager-Previews'; TagPattern = '\Apreview-v[0-9]+\z' }
}
$script:CredentialPath = 'B:\APMM-next-work\credentials\github-token.txt'

function Get-LocalReleasePlan {
    if (-not [IO.Path]::IsPathFullyQualified($PlanPath) -or -not (Test-Path -LiteralPath $PlanPath -PathType Leaf)) {
        throw 'PlanPath 必须是存在的绝对 publication-plan.json 路径。'
    }

    try {
        $plan = Get-Content -LiteralPath $PlanPath -Raw -Encoding UTF8 | ConvertFrom-Json -AsHashtable
    } catch {
        throw '无法解析 publication-plan.json。请重新生成并检查本轮发布计划。'
    }

    $target = $script:RepositoryTargets[$Repository]
    if (-not $plan.github -or -not $plan.github[$target.Key]) {
        throw "发布计划中没有 $Repository 附件计划。"
    }
    $release = $plan.github[$target.Key]
    if ($release.repository -cne $target.Slug -or $release.tag -cnotmatch $target.TagPattern) {
        throw '发布计划仓库或标签与固定目标不符。'
    }
    if (-not [IO.Path]::IsPathFullyQualified([string]$release.directory) -or
        -not (Test-Path -LiteralPath $release.directory -PathType Container)) {
        throw '发布计划中的暂存目录必须是存在的绝对路径。'
    }
    if (-not $release.attachments -or $release.attachments.Count -eq 0) {
        throw '发布计划没有附件。'
    }
    if (-not $plan.files) { throw '发布计划缺少附件摘要清单。' }

    $stage = [IO.Path]::TrimEndingDirectorySeparator((Get-Item -LiteralPath $release.directory).FullName)
    $expectedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($name in $release.attachments) {
        if ($name -isnot [string] -or $name -notmatch '\A[A-Za-z0-9][A-Za-z0-9._-]*\z' -or
            -not $expectedNames.Add($name)) {
            throw '附件清单含无效或重复文件名。'
        }
    }

    $notesFile = [string]$release.notesFile
    if ($notesFile -and ($notesFile -notmatch '\A[A-Za-z0-9][A-Za-z0-9._-]*\z' -or $expectedNames.Contains($notesFile))) {
        throw '发布说明文件名无效或被重复列为附件。'
    }

    $stageChildren = @(Get-ChildItem -LiteralPath $stage -Force)
    if (@($stageChildren | Where-Object { $_.PSIsContainer }).Count -gt 0) {
        throw 'GitHub Release 暂存目录必须是扁平文件集合。'
    }
    $actualFiles = @($stageChildren | Where-Object { -not $_.PSIsContainer })
    $actualNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $actualFiles) {
        if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw '暂存目录不能包含符号链接或重解析点。'
        }
        $null = $actualNames.Add($file.Name)
        if (-not $expectedNames.Contains($file.Name) -and $file.Name -cne $notesFile) {
            throw "暂存目录含未列入发布计划的文件：$($file.Name)"
        }
    }
    if ($notesFile -and -not $actualNames.Contains($notesFile)) {
        throw '发布计划指定的 RELEASE-NOTES.md 不在暂存目录中。'
    }

    $plannedFiles = @{}
    foreach ($entry in $plan.files) {
        if ($entry.channel -ceq 'github') {
            if ($plannedFiles.ContainsKey([string]$entry.path)) { throw '发布计划包含重复附件路径。' }
            $plannedFiles[[string]$entry.path] = $entry
        }
    }
    $releasePrefix = '{0}/{1}/' -f $target.Key, $release.tag
    $assets = foreach ($name in $release.attachments) {
        $path = Join-Path $stage $name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "计划附件不存在：$name"
        }
        $file = Get-Item -LiteralPath $path -Force
        if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint -or $file.Length -le 0 -or $file.Length -gt [int]::MaxValue) {
            throw "计划附件必须是非空的普通文件：$name"
        }
        $hash = Get-FileHash -LiteralPath $path -Algorithm SHA256
        $digest = $hash.Hash.ToLowerInvariant()
        $relativePath = $releasePrefix + $name
        if (-not $plannedFiles.ContainsKey($relativePath)) { throw "附件不在本轮摘要清单中：$name" }
        $record = $plannedFiles[$relativePath]
        if ([long]$record.bytes -ne $file.Length -or ([string]$record.sha256).ToLowerInvariant() -cne $digest) {
            throw "本地附件与本轮计划摘要不符：$name"
        }
        [pscustomobject]@{ Name = $name; Path = $file.FullName; Bytes = [long]$file.Length; Sha256 = $digest }
    }
    if ($plannedFiles.Count -ne $release.attachments.Count + [int][bool]$notesFile) {
        throw '发布计划的 GitHub 文件清单与目标 Release 附件数量不一致。'
    }

    [pscustomobject]@{
        Repository = $Repository
        Slug = $target.Slug
        Tag = [string]$release.tag
        Directory = $stage
        Assets = @($assets)
    }
}

function Read-GitHubToken {
    try { $content = [IO.File]::ReadAllText($script:CredentialPath) }
    catch { throw '无法读取工作盘 GitHub 凭据文件。请检查路径和文件权限。' }

    $records = [regex]::Matches($content, '(?im)^\s*Token:[ \t]*([^\r\n]+)\s*$')
    if ($records.Count -eq 1) {
        $token = $records[0].Groups[1].Value.Trim()
    } elseif ($records.Count -gt 1) {
        throw 'GitHub 凭据文件包含多条 Token 记录，拒绝使用。'
    } else {
        $lines = @($content -split '\r?\n' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
        if ($lines.Count -ne 1) { throw 'GitHub 凭据文件必须是单行令牌或包含唯一 Token: 记录。' }
        $token = $lines[0]
    }
    if ($token -notmatch '\A[!-~]+\z') { throw 'GitHub 令牌格式无效，拒绝使用。' }
    return $token
}

function Invoke-Gh {
    param([string]$GhPath, [string[]]$Arguments, [string]$Token, [string]$WorkingDirectory)

    $start = [Diagnostics.ProcessStartInfo]::new($GhPath)
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.CreateNoWindow = $true
    $start.WorkingDirectory = $WorkingDirectory
    $start.StandardOutputEncoding = [Text.Encoding]::UTF8
    $start.StandardErrorEncoding = [Text.Encoding]::UTF8
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }

    foreach ($name in @($start.Environment.Keys)) {
        if ($name -in @('GH_TOKEN', 'GITHUB_TOKEN', 'GH_ENTERPRISE_TOKEN', 'GITHUB_ENTERPRISE_TOKEN',
                        'GH_DEBUG', 'DEBUG', 'GH_HOST', 'GH_REPO', 'GH_PAGER', 'PAGER')) {
            $null = $start.Environment.Remove($name)
        }
    }
    $start.Environment['GH_TOKEN'] = $Token
    $start.Environment['GH_PROMPT_DISABLED'] = '1'
    $start.Environment['GH_NO_UPDATE_NOTIFIER'] = '1'

    $process = [Diagnostics.Process]::new()
    try {
        $process.StartInfo = $start
        if (-not $process.Start()) { throw '无法启动 GitHub CLI。' }
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        return @{
            ExitCode = $process.ExitCode
            Stdout = $stdout.GetAwaiter().GetResult()
            Stderr = $stderr.GetAwaiter().GetResult()
        }
    } catch {
        throw 'GitHub CLI 启动或执行失败。详细进程输出已隐藏，避免泄漏凭据或请求信息。'
    } finally {
        $process.Dispose()
    }
}

function Invoke-GhJson {
    param([string]$GhPath, [string[]]$Arguments, [string]$Token, [string]$WorkingDirectory)
    $result = Invoke-Gh $GhPath $Arguments $Token $WorkingDirectory
    if ($result.ExitCode -ne 0) { throw "GitHub CLI 请求失败，退出码 $($result.ExitCode)。原始输出已隐藏。" }
    try { return $result.Stdout | ConvertFrom-Json -AsHashtable }
    catch { throw 'GitHub CLI 返回了无法解析的 JSON。原始输出已隐藏。' }
}

function Get-Release {
    param([string]$GhPath, [string]$Token, [string]$Slug, [string]$Tag, [string]$Directory)
    $endpoint = 'repos/{0}/releases/tags/{1}' -f $Slug, $Tag
    $release = Invoke-GhJson $GhPath @('api', $endpoint) $Token $Directory
    if ($release.tag_name -cne $Tag) { throw 'GitHub 返回的发行标签与请求不一致。' }
    if ($release.draft -ne $true) { throw '目标 GitHub Release 必须存在且保持为草稿。' }
    return $release
}

function Assert-RemoteAsset {
    param($RemoteAsset, $LocalAsset)
    if ($null -eq $RemoteAsset) { throw "上传后未找到远端附件：$($LocalAsset.Name)" }
    if ($RemoteAsset.state -cne 'uploaded' -or $RemoteAsset.digest -notmatch '\Asha256:[0-9a-fA-F]{64}\z' -or
        $RemoteAsset.size -isnot [long] -and $RemoteAsset.size -isnot [int]) {
        throw "远端附件缺少已完成状态、SHA-256 digest 或 size：$($LocalAsset.Name)"
    }
    $remoteHash = ([string]$RemoteAsset.digest).Substring(7).ToLowerInvariant()
    if ($remoteHash -cne $LocalAsset.Sha256 -or [long]$RemoteAsset.size -ne $LocalAsset.Bytes) {
        throw "远端附件与本地摘要不符：$($LocalAsset.Name)"
    }
}

if ($Check -and $Execute) { throw '不能同时指定 -Check 与 -Execute。' }
$localPlan = Get-LocalReleasePlan
$summary = [ordered]@{
    Repository = $localPlan.Repository
    Slug = $localPlan.Slug
    Tag = $localPlan.Tag
    Stage = $localPlan.Directory
    Assets = @($localPlan.Assets | ForEach-Object { [ordered]@{ Name = $_.Name; Bytes = $_.Bytes; Sha256 = $_.Sha256 } })
}
if (-not $Execute) {
    [pscustomobject]$summary
    return
}

$gh = Get-Command gh.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $gh) { throw '未找到 gh.exe。安装 GitHub CLI 后重试。' }
$token = Read-GitHubToken
try {
    $release = Get-Release $gh.Source $token $localPlan.Slug $localPlan.Tag $localPlan.Directory
    $remoteAssets = @{}
    foreach ($asset in @($release.assets)) {
        if ($remoteAssets.ContainsKey([string]$asset.name)) { throw '目标 Release 返回了重复附件名，停止上传。' }
        $remoteAssets[[string]$asset.name] = $asset
    }

    $pending = [Collections.Generic.List[object]]::new()
    $completed = [Collections.Generic.List[object]]::new()
    foreach ($asset in $localPlan.Assets) {
        if ($remoteAssets.ContainsKey($asset.Name)) {
            $existing = $remoteAssets[$asset.Name]
            Assert-RemoteAsset $existing $asset
            $completed.Add([pscustomobject]@{ Name = $asset.Name; State = 'AlreadyVerified'; Bytes = $asset.Bytes; Sha256 = $asset.Sha256 })
        } else {
            $pending.Add($asset)
        }
    }

    foreach ($asset in $pending) {
        $currentHash = (Get-FileHash -LiteralPath $asset.Path -Algorithm SHA256).Hash.ToLowerInvariant()
        $currentBytes = (Get-Item -LiteralPath $asset.Path).Length
        if ($currentHash -cne $asset.Sha256 -or [long]$currentBytes -ne $asset.Bytes) {
            throw "本地附件在预检后发生变化：$($asset.Name)"
        }

        $upload = Invoke-Gh $gh.Source @('release', 'upload', $localPlan.Tag, $asset.Path, '--repo', $localPlan.Slug) $token $localPlan.Directory
        if ($upload.ExitCode -ne 0) { throw "附件上传失败：$($asset.Name)，退出码 $($upload.ExitCode)。可重跑脚本继续；原始输出已隐藏。" }

        $verifiedRelease = Get-Release $gh.Source $token $localPlan.Slug $localPlan.Tag $localPlan.Directory
        $verified = @($verifiedRelease.assets | Where-Object { $_.name -ceq $asset.Name })
        if ($verified.Count -ne 1) { throw "上传后远端附件数量异常：$($asset.Name)" }
        Assert-RemoteAsset $verified[0] $asset
        $completed.Add([pscustomobject]@{ Name = $asset.Name; State = 'UploadedAndVerified'; Bytes = $asset.Bytes; Sha256 = $asset.Sha256 })
    }

    $summary.Assets = @($completed)
    $summary.State = 'AllAssetsVerified'
    [pscustomobject]$summary
} finally {
    Remove-Variable -Name token -ErrorAction SilentlyContinue
}
