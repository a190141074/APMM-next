#requires -Version 7.2
<#
.SYNOPSIS
核对或推送 APMM Next 的一个 CNB main 提交或版本标签。
.DESCRIPTION
默认只读核对。-Execute 会写入选定的 CNB 仓库，使用前需要取得本次远端写入授权。
令牌仅在确有改动需要推送时读取，并通过隔离的 Git 子进程环境传递。
#>
[CmdletBinding(DefaultParameterSetName = 'Main')]
param(
    [Parameter(Mandatory)]
    [ValidateSet('Source', 'Assets')]
    [string]$Repository,
    [Parameter(Mandatory)]
    [string]$Checkout,
    [Parameter(Mandatory, ParameterSetName = 'Main')]
    [ValidatePattern('\A[0-9a-fA-F]{40}\z')]
    [string]$Commit,
    [Parameter(Mandatory, ParameterSetName = 'Tag')]
    [string]$Tag,
    [switch]$Execute
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$gitExe = (Get-Command git.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
$workRoot = 'B:\APMM-next-work'
$credentialPath = Join-Path $workRoot 'credentials\cnb-token.txt'
$target = if ($Repository -eq 'Source') {
    @{ Remote = 'cnb'; Url = 'https://cnb.cool/astral-weird-egg/APMM-next.git' }
} else {
    @{ Remote = 'origin'; Url = 'https://cnb.cool/astral-weird-egg/astral-party-mod-manager-assets.git' }
}

function Invoke-Git {
    param([string]$Directory, [string[]]$GitArgs, [switch]$Isolated, [string]$Authorization)
    $start = [Diagnostics.ProcessStartInfo]::new($gitExe)
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.CreateNoWindow = $true
    $start.WorkingDirectory = $Directory
    $start.StandardOutputEncoding = [Text.Encoding]::UTF8
    $start.StandardErrorEncoding = [Text.Encoding]::UTF8
    foreach ($arg in $GitArgs) { $start.ArgumentList.Add($arg) }
    foreach ($name in @($start.Environment.Keys)) {
        if ($name -like 'GIT_*' -or $name -like 'GCM_*') { $null = $start.Environment.Remove($name) }
    }
    $start.Environment['GIT_TERMINAL_PROMPT'] = '0'
    $start.Environment['GCM_INTERACTIVE'] = 'never'
    $start.Environment['GIT_OPTIONAL_LOCKS'] = '0'
    if ($Isolated) {
        $start.Environment['GIT_CONFIG_NOSYSTEM'] = '1'
        $start.Environment['GIT_CONFIG_GLOBAL'] = 'NUL'
        $settings = [ordered]@{
            'credential.helper' = ''
            'credential.interactive' = 'never'
            'http.followRedirects' = 'false'
            'protocol.allow' = 'never'
            'protocol.https.allow' = 'always'
        }
        if ($Authorization) { $settings['http.' + $target.Url + '.extraHeader'] = $Authorization }
        $start.Environment['GIT_CONFIG_COUNT'] = [string]$settings.Count
        $index = 0
        foreach ($key in $settings.Keys) {
            $start.Environment["GIT_CONFIG_KEY_$index"] = $key
            $start.Environment["GIT_CONFIG_VALUE_$index"] = $settings[$key]
            $index++
        }
    }
    $process = [Diagnostics.Process]::new()
    try {
        $process.StartInfo = $start
        if (-not $process.Start()) { throw '无法启动 Git。' }
        $outputTask = $process.StandardOutput.ReadToEndAsync()
        $errorTask = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $outputText = $outputTask.GetAwaiter().GetResult()
        $errorText = $errorTask.GetAwaiter().GetResult()
        $authError = $errorText -match 'requested URL returned error: (401|403)|HTTP Basic: Access denied|authentication failed|could not read Username'
        return @{ ExitCode = $process.ExitCode; Output = $outputText.Trim(); AuthError = $authError }
    } finally { $process.Dispose() }
}

function Read-CnbToken {
    try { $raw = [IO.File]::ReadAllText($credentialPath).Trim() }
    catch { throw '无法读取工作盘凭据文件。请检查受控凭据路径和访问权限。' }
    $records = [regex]::Matches($raw, '(?im)^\s*Token:[ \t]*([^\r\n]+)\s*$')
    if ($records.Count -eq 1) { $raw = $records[0].Groups[1].Value.Trim() }
    if ($records.Count -gt 1 -or $raw -notmatch '\A[A-Za-z0-9._~+/=-]+\z') {
        throw 'CNB 凭据文件应包含单行令牌或唯一的 Token: 记录。'
    }
    return $raw
}

function Get-RemoteObject {
    param([string]$Directory, [string]$Reference)
    $result = Invoke-Git $Directory @('ls-remote', '--refs', '--', $target.Url, $Reference) -Isolated
    if ($result.ExitCode -ne 0) { throw '无法回读 CNB 引用。请检查 CNB 访问状态和网络。' }
    if (-not $result.Output) { return $null }
    $fields = $result.Output.Split([char]9)
    if ($fields.Count -ne 2 -or $fields[0] -notmatch '\A[0-9a-f]{40}\z' -or $fields[1] -cne $Reference) {
        throw 'CNB 返回了无法核对的 Git 引用。'
    }
    return $fields[0]
}

if (-not [IO.Path]::IsPathFullyQualified($Checkout) -or -not (Test-Path -LiteralPath $Checkout -PathType Container)) {
    throw 'Checkout 必须指定存在的绝对仓库根路径。'
}
$checkoutPath = [IO.Path]::TrimEndingDirectorySeparator((Get-Item -LiteralPath $Checkout).FullName)
$root = Invoke-Git $checkoutPath @('rev-parse', '--show-toplevel')
if ($root.ExitCode -ne 0 -or [IO.Path]::GetFullPath($root.Output) -ine $checkoutPath) { throw 'Checkout 必须是 Git 工作区的根目录。' }
$remote = Invoke-Git $checkoutPath @('remote', 'get-url', '--push', '--all', $target.Remote)
if ($remote.ExitCode -ne 0 -or $remote.Output -cne $target.Url) { throw '仓库的推送地址与选定的 CNB 目标不符。' }

if ($PSCmdlet.ParameterSetName -eq 'Tag') {
    $number = '(?:0|[1-9][0-9]*)'
    $pre = '(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)'
    if ($Tag -cnotmatch ("\Av$number\.$number\.$number(?:-$pre(?:\.$pre)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?\z")) { throw 'Tag 必须是以小写 v 开头的 SemVer 版本。' }
    $reference = "refs/tags/$Tag"
    $localObject = Invoke-Git $checkoutPath @('show-ref', '--verify', '--hash', $reference)
    $localCommit = Invoke-Git $checkoutPath @('rev-parse', '--verify', ('{0}^{{commit}}' -f $reference))
    if ($localObject.ExitCode -ne 0 -or $localCommit.ExitCode -ne 0) { throw '指定标签不存在或未指向提交。' }
    $desired = $localObject.Output
} else {
    $reference = 'refs/heads/main'
    $localCommit = Invoke-Git $checkoutPath @('cat-file', '-t', $Commit)
    if ($localCommit.ExitCode -ne 0 -or $localCommit.Output -cne 'commit') { throw '指定 SHA 必须指向本地提交。' }
    $desired = $Commit.ToLowerInvariant()
}
if ($desired -cnotmatch '\A[0-9a-f]{40}\z') { throw '目标必须使用 SHA-1 对象。' }

$objects = Invoke-Git $checkoutPath @('rev-parse', '--path-format=absolute', '--git-path', 'objects')
$shallow = Invoke-Git $checkoutPath @('rev-parse', '--path-format=absolute', '--git-path', 'shallow')
if ($objects.ExitCode -ne 0 -or $shallow.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $objects.Output -PathType Container)) { throw '无法定位现有仓库的对象目录。' }

$scratch = Join-Path $workRoot ('cnb-push-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $scratch -ErrorAction Stop
try {
    $init = Invoke-Git $scratch @('init', '--quiet', '--bare', '--object-format=sha1', '--template=') -Isolated
    if ($init.ExitCode -ne 0) { throw '无法创建隔离 Git 仓库。' }
    [IO.File]::WriteAllText((Join-Path $scratch 'objects/info/alternates'), $objects.Output + [Environment]::NewLine)
    if (Test-Path -LiteralPath $shallow.Output -PathType Leaf) { Copy-Item -LiteralPath $shallow.Output -Destination (Join-Path $scratch 'shallow') }

    $before = Get-RemoteObject $scratch $reference
    $result = [ordered]@{ Repository = $Repository; Url = $target.Url; Ref = $reference; Object = $desired; RemoteBefore = $before; State = '' }
    if ($before -eq $desired) {
        $result.State = 'NoChange'
    } else {
        if ($PSCmdlet.ParameterSetName -eq 'Main') {
            if (-not $before) { throw 'CNB main 不存在，请先核对目标仓库。' }
            $ancestor = Invoke-Git $scratch @('merge-base', '--is-ancestor', $before, $desired) -Isolated
            if ($ancestor.ExitCode -ne 0) { throw '无法证明 main 可快进。请获取 CNB 最新 main 和所需历史后重试。' }
        } elseif ($before) { throw 'CNB 同名标签指向其他对象，请核对正确版本标签。' }

        if ($Execute) {
            $token = Read-CnbToken
            $pair = [Text.Encoding]::UTF8.GetBytes('cnb:' + $token)
            try {
                $authorization = 'Authorization: Basic ' + [Convert]::ToBase64String($pair)
                $refspec = '{0}:{1}' -f $desired, $reference
                $push = Invoke-Git $scratch @('push', '--porcelain', '--quiet', '--', $target.Url, $refspec) -Isolated -Authorization $authorization
            } finally {
                [Array]::Clear($pair, 0, $pair.Length)
                Remove-Variable -Name token, authorization -ErrorAction SilentlyContinue
            }
            if ($push.ExitCode -ne 0) {
                if ($push.AuthError) { throw 'CNB 拒绝鉴权，请检查访问令牌是否有效并具备对应仓库的写权限。' }
                throw 'CNB 推送失败，请回读目标引用并检查网络或远端分支规则。'
            }
            $after = Get-RemoteObject $scratch $reference
            if ($after -ne $desired) { throw '推送已返回成功，但公网回读的引用与目标不同，请核对远端。' }
            $result.State = 'Pushed'
            $result.RemoteAfter = $after
        } else { $result.State = 'Ready' }
    }
    [pscustomobject]$result
} finally {
    if (Test-Path -LiteralPath $scratch -PathType Container) {
        $resolved = (Resolve-Path -LiteralPath $scratch).ProviderPath
        if ([IO.Path]::GetDirectoryName($resolved) -ieq $workRoot -and [IO.Path]::GetFileName($resolved) -match '\Acnb-push-[0-9a-f]{32}\z') {
            Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
