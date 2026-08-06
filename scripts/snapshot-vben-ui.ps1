# 将 dev/vben-ui 快照到 src-tauri/templates/ruoyi-vue/ui/vben-web-ele/
# 排除 node_modules / 构建缓存，并写入锻造台占位符，供 replace_ui 任务替换。
#
# 用法（仓库根目录）：
#   powershell -ExecutionPolicy Bypass -File scripts/snapshot-vben-ui.ps1

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Src = Join-Path $Root 'dev\vben-ui'
$Dest = Join-Path $Root 'src-tauri\templates\ruoyi-vue\ui\vben-web-ele'

if (-not (Test-Path $Src)) {
    throw "源目录不存在：$Src"
}

Write-Host "快照源：$Src"
Write-Host "快照目标：$Dest"

if (Test-Path $Dest) {
    Write-Host "清理旧模板目录..."
    Remove-Item -LiteralPath $Dest -Recurse -Force
}
New-Item -ItemType Directory -Path $Dest -Force | Out-Null

# robocopy：复制源码，排除大体积/缓存目录
$ExcludeDirs = @(
    'node_modules', '.turbo', 'dist', '.git', '.cache', '.nitro', '.output',
    'coverage', '.pnpm-store', 'playwright-report', 'test-results'
)
$xdArgs = @()
foreach ($d in $ExcludeDirs) { $xdArgs += @('/XD', $d) }

Write-Host "正在复制文件（排除 node_modules 等）..."
# robocopy 成功退出码为 0-7
& robocopy $Src $Dest /E /NFL /NDL /NJH /NJS /nc /ns /np @xdArgs | Out-Null
$rc = $LASTEXITCODE
if ($rc -ge 8) {
    throw "robocopy 失败，退出码 $rc"
}

function Set-Utf8NoBomContent([string]$Path, [string]$Content) {
    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

function Replace-InFile([string]$RelPath, [hashtable]$Replacements) {
    $full = Join-Path $Dest $RelPath
    if (-not (Test-Path $full)) {
        Write-Warning "跳过（文件不存在）：$RelPath"
        return
    }
    $text = [System.IO.File]::ReadAllText($full)
    $original = $text
    foreach ($key in $Replacements.Keys) {
        $text = $text.Replace($key, $Replacements[$key])
    }
    if ($text -ne $original) {
        Set-Utf8NoBomContent $full $text
        Write-Host "  占位符：$RelPath"
    }
}

Write-Host "写入占位符..."

# 应用标题 / 命名空间（用正则，避免中文编码比对失败）
$envPath = Join-Path $Dest 'apps\web-ele\.env'
if (Test-Path $envPath) {
    $envText = [System.IO.File]::ReadAllText($envPath)
    $envNew = [regex]::Replace($envText, '(?m)^VITE_APP_TITLE=.*$', 'VITE_APP_TITLE={{FRONTEND_TITLE}}')
    # 按模块前缀隔离 localStorage，避免仍读到「Vben Admin Ele」等旧偏好
    $envNew = [regex]::Replace($envNew, '(?m)^VITE_APP_NAMESPACE=.*$', 'VITE_APP_NAMESPACE={{MODULE_PREFIX}}-web-ele')
    if ($envNew -ne $envText) {
        Set-Utf8NoBomContent $envPath $envNew
        Write-Host "  占位符：apps\web-ele\.env"
    }
}

# 开发代理目标（本地联调默认 14001 → 占位符）
Replace-InFile 'apps\web-ele\vite.config.mts' @{
    "target: 'http://localhost:14001'" = "target: '{{API_BASE_URL_DEV}}'"
}

# 版权偏好（正则替换，避免中文编码问题）
$prefPath = Join-Path $Dest 'apps\web-ele\src\preferences.ts'
if (Test-Path $prefPath) {
    $prefText = [System.IO.File]::ReadAllText($prefPath)
    $prefNew = $prefText
    $prefNew = [regex]::Replace($prefNew, "companyName:\s*'[^']*'", "companyName: '{{COPYRIGHT_HOLDER}}'")
    $prefNew = [regex]::Replace($prefNew, "date:\s*'[^']*'", "date: '{{COPYRIGHT_YEAR}}'")
    if ($prefNew -ne $prefText) {
        Set-Utf8NoBomContent $prefPath $prefNew
        Write-Host "  占位符：apps\web-ele\src\preferences.ts"
    }
}

# 模板说明（用 .NET UTF-8 无 BOM 写入，避免 PowerShell 默认编码乱码）
$readmeLines = @(
    '# vben-web-ele',
    '',
    '基于 vue-vben-admin（web-ele）适配若依后端的预置后台 UI 模板。',
    '',
    '由 ``scripts/snapshot-vben-ui.ps1`` 从 ``dev/vben-ui`` 快照生成。',
    '执行改造时，``replace_ui`` 会复制本目录到 ``{prefix}-ui/`` 并替换占位符：',
    '',
    '| 占位符 | 含义 |',
    '|--------|------|',
    '| ``{{FRONTEND_TITLE}}`` | 前端标题（VITE_APP_TITLE） |',
    '| ``{{MODULE_PREFIX}}`` | 模块前缀（同时用于 VITE_APP_NAMESPACE） |',
    '| ``{{API_BASE_URL_DEV}}`` | 开发环境后端地址（vite proxy） |',
    '| ``{{COPYRIGHT_HOLDER}}`` | 版权方 |',
    '| ``{{COPYRIGHT_YEAR}}`` | 版权年份 |',
    '| ``{{PROJECT_NAME}}`` / ``{{SERVER_PORT}}`` | 其它通用占位 |',
    '',
    '生成后请在 ``{prefix}-ui`` 目录执行：',
    '',
    '```bash',
    'pnpm install',
    'pnpm run dev:ele',
    'pnpm run build:ele',
    '```',
    ''
)
Set-Utf8NoBomContent (Join-Path $Dest 'README.md') ($readmeLines -join "`n")

$fileCount = (Get-ChildItem $Dest -Recurse -File | Measure-Object).Count
Write-Host "完成：共 $fileCount 个文件 → $Dest"
