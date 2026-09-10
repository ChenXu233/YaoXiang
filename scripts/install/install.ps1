# install.ps1 — YaoXiang 一行命令安装（RFC-037 傻瓜渠道，Windows）
#
# irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
#
# 行为：下载最新重组发行包 → 解压进 $YAOXIANG_HOME\versions\<ver>\ →
# bin\yx.exe 就位 $YAOXIANG_HOME\bin\ → 写 settings.toml 默认版本 → 用户 PATH。

$ErrorActionPreference = "Stop"

$Repo = "ChenXu233/YaoXiang"
$Target = "x86_64-pc-windows-msvc"
$YxHome = if ($env:YAOXIANG_HOME) { $env:YAOXIANG_HOME } else { Join-Path $env:USERPROFILE ".yaoxiang" }

# 版本号：环境变量优先，否则取 GitHub 最新 stable
if ($env:YAOXIANG_VERSION) {
    $Version = $env:YAOXIANG_VERSION.TrimStart("v")
}
else {
    $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $rel.tag_name.TrimStart("v")
}

# 下载 + 解压
$Asset = "yaoxiang-$Version-$Target.zip"
$Url = "https://github.com/$Repo/releases/download/v$Version/$Asset"
$Tmp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
Write-Host "yx: downloading $Url"
Invoke-WebRequest -Uri $Url -OutFile (Join-Path $Tmp $Asset)

# 校验 .sha256 旁证（缺失时降级为提示，与 yx 行为一致）
$ShaExpected = $null
try {
    $ShaExpected = (Invoke-WebRequest -Uri "$Url.sha256" -UseBasicParsing).Content.Trim().Split()[0]
}
catch {
    Write-Host "yx: warning: .sha256 not available, skipping verification"
}
if ($ShaExpected) {
    Write-Host "yx: verifying checksum"
    $ShaActual = (Get-FileHash -Path (Join-Path $Tmp $Asset) -Algorithm SHA256).Hash.ToLower()
    if ($ShaExpected -ne $ShaActual) {
        # throw 而非 exit：irm|iex 场景下 exit 会终止用户当前 PowerShell 会话
        throw "yx: checksum mismatch"
    }
}

Expand-Archive -Path (Join-Path $Tmp $Asset) -DestinationPath (Join-Path $Tmp "out")

# 就位：版本目录 = 发行包解压根；bin\yx.exe 入口
New-Item -ItemType Directory -Force -Path (Join-Path $YxHome "versions\$Version"), (Join-Path $YxHome "bin") | Out-Null
Copy-Item -Recurse -Force -Path (Join-Path $Tmp "out\yaoxiang-$Version-$Target\*") -Destination (Join-Path $YxHome "versions\$Version")
Copy-Item -Force (Join-Path $YxHome "versions\$Version\bin\yx.exe") (Join-Path $YxHome "bin\yx.exe")

# 首次安装写默认版本，不覆盖既有设置
$Settings = Join-Path $YxHome "settings.toml"
if (-not (Test-Path $Settings)) {
    Set-Content -Path $Settings -Value "default = `"$Version`""
}

# 用户级 PATH
$BinDir = Join-Path $YxHome "bin"
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($UserPath -split ';') -notcontains $BinDir) {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$BinDir", "User")
    Write-Host "yx: added $BinDir to user PATH (restart shell to take effect)"
}

Remove-Item -Recurse -Force $Tmp
Write-Host "yx: installed $Version -> $YxHome\versions\$Version"
Write-Host "yx: run 'yx' to use, 'yx toolchain update' to upgrade"
