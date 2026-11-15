# 管理展开仅把 MSI 内容提取到独立工作目录，不注册产品或覆盖现有安装。
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$report = Get-Content -LiteralPath (Join-Path $projectRoot 'verification-distributions.json') -Raw | ConvertFrom-Json
$package = ($report.artifacts | Where-Object kind -eq 'msi').path
$work = if ($env:APMM_WORK_DIR) { $env:APMM_WORK_DIR } else { 'B:\APMM-next-work' }
$area = Join-Path $work ('msi-validation\' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $area -Force | Out-Null
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase($package, 0)
function Read-MsiProperty([string]$name) {
    $view = $database.OpenView("SELECT ``Value`` FROM ``Property`` WHERE ``Property`` = '$name'")
    [void]$view.Execute()
    $record = $view.Fetch()
    if (-not $record) { throw "MSI 缺少属性 $name" }
    $value = $record.StringData(1)
    [void]$view.Close()
    return $value
}
$product = Read-MsiProperty 'ProductName'
$version = Read-MsiProperty 'ProductVersion'
$upgrade = Read-MsiProperty 'UpgradeCode'
if ($product -ne 'APMM' -or $version -ne $report.version -or $upgrade.Trim('{}') -ne 'DF7031A2-E41A-465C-BCBA-4DF3FCB73894') { throw 'MSI 产品身份不匹配' }
$target = Join-Path $area 'extracted'
$log = Join-Path $area 'administrative-extraction.log'
$process = Start-Process msiexec.exe -ArgumentList @('/a', ('"' + $package + '"'), '/qn', ('TARGETDIR="' + $target + '"'), '/l*v', ('"' + $log + '"')) -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0) { throw "管理展开失败 $($process.ExitCode)：$log" }
$executables = @(Get-ChildItem -LiteralPath $target -Filter astral-mod-manager.exe -Recurse -File)
if ($executables.Count -ne 1) { throw 'MSI 主程序数量不正确' }
$installRoot = $executables[0].DirectoryName
$identity = Get-Content -LiteralPath (Join-Path $installRoot 'installation.json') -Raw | ConvertFrom-Json
if ($identity.kind -ne 'msi') { throw '安装身份错误' }
foreach ($file in $report.msiFiles) {
    $path = Join-Path $installRoot $file.path
    if ((Get-Item -LiteralPath $path).Length -ne $file.bytes -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) { throw "MSI 内容不一致：$($file.path)" }
}
$result = [ordered]@{ version=$version; area=$area; installRoot=$installRoot; product=$product; upgradeCode=$upgrade; filesVerified=$report.msiFiles.Count; administrativeExtraction=$true; registeredInstallTested=$false; signature=(Get-AuthenticodeSignature -LiteralPath $package).Status.ToString() }
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $projectRoot 'verification-msi.json') -Encoding utf8
$result | ConvertTo-Json -Depth 5
