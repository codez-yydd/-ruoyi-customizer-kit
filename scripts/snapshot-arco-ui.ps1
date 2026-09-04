# 将 dev/arco-ui 快照到 src-tauri/templates/ruoyi-vue/ui/arco/
# 排除 node_modules / dist / 锁文件，并写入锻造台占位符，供 replace_ui 任务替换。
# 与 scripts/snapshot-arco-ui.sh 逻辑一致（macOS/Linux 用 sh 版）。
#
# 用法（仓库根目录）：
#   powershell -ExecutionPolicy Bypass -File scripts/snapshot-arco-ui.ps1

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Src = Join-Path $Root 'dev\arco-ui'
$Dest = Join-Path $Root 'src-tauri\templates\ruoyi-vue\ui\arco'

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

# robocopy：复制源码，排除大体积/缓存/锁文件
# （package-lock.json 不进模板：产物由锻造台生成后 npm install 重建锁文件）
$ExcludeDirs = @('node_modules', 'dist', '.git', 'logs', 'uploadPath')
$ExcludeFiles = @('package-lock.json', '.DS_Store')
$xdArgs = @()
foreach ($d in $ExcludeDirs) { $xdArgs += @('/XD', $d) }
$xfArgs = @()
foreach ($f in $ExcludeFiles) { $xfArgs += @('/XF', $f) }

Write-Host "正在复制文件（排除 node_modules 等）..."
# robocopy 成功退出码为 0-7
& robocopy $Src $Dest /E /NFL /NDL /NJH /NJS /nc /ns /np @xdArgs @xfArgs | Out-Null
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

# 应用标题（用正则，避免中文编码比对失败）
$envPath = Join-Path $Dest '.env'
if (Test-Path $envPath) {
    $envText = [System.IO.File]::ReadAllText($envPath)
    $envNew = [regex]::Replace($envText, '(?m)^VITE_APP_TITLE=.*$', 'VITE_APP_TITLE={{FRONTEND_TITLE}}')
    if ($envNew -ne $envText) {
        Set-Utf8NoBomContent $envPath $envNew
        Write-Host "  占位符：.env"
    }
}

# 开发代理目标：/api、/v3/api-docs、/webjars 三段 proxy 同源同后端，统一字面替换
Replace-InFile 'vite.config.ts' @{
    'http://localhost:14001' = '{{API_BASE_URL_DEV}}'
}

# 页脚版权（本地联调默认值 → 占位符）
Replace-InFile 'src\layouts\index.vue' @{
    "const COPYRIGHT_YEAR = '2026'" = "const COPYRIGHT_YEAR = '{{COPYRIGHT_YEAR}}'"
    "const COPYRIGHT_HOLDER = 'RuoYi'" = "const COPYRIGHT_HOLDER = '{{COPYRIGHT_HOLDER}}'"
}

# 包名（dev 用名 → 模块前缀）
Replace-InFile 'package.json' @{
    '"name": "ruoyi-arco-admin"' = '"name": "{{MODULE_PREFIX}}-ui"'
}

# 模板说明（用 .NET UTF-8 无 BOM 写入，避免 PowerShell 默认编码乱码）
$readmeLines = @(
    '# arco',
    '',
    '基于 Arco Design Vue 适配若依后端的预置后台 UI 模板（npm 单包工程，非 monorepo）。',
    '',
    '由 ``scripts/snapshot-arco-ui.ps1``（macOS/Linux 用 ``scripts/snapshot-arco-ui.sh``）从 ``dev/arco-ui`` 快照生成。',
    '执行改造时，``replace_ui`` 会复制本目录到 ``{prefix}-ui/`` 并替换占位符。',
    '',
    '## 技术栈',
    '',
    'Vue 3 + TypeScript + Vite 6 + Pinia + Arco Design Vue（Node >= 20，npm 单包工程）。',
    '',
    '## 常用命令',
    '',
    '在 ``{prefix}-ui`` 目录下：',
    '',
    '```bash',
    'npm install',
    'npm run dev',
    'npm run build:prod',
    '```',
    '',
    '也可在项目根目录使用锻造台生成的 ``run-ui`` / ``build`` 脚本（无 pnpm-workspace.yaml 时自动走 npm 分支）。',
    '',
    '## 环境变量',
    '',
    '| 文件 | 变量 | 说明 |',
    '|------|------|------|',
    '| ``.env`` | ``VITE_APP_TITLE`` | 应用标题 |',
    '| ``.env`` | ``VITE_APP_REGISTER`` | 注册入口开关：``true`` 登录页显示注册入口（后端注册开关由参数 ``sys.account.registerUser`` 控制，关闭时提交注册会提示「当前系统没有开启注册功能」）；``false`` 强制隐藏注册入口且 ``/register`` 重定向登录页 |',
    '| ``.env.development`` | ``VITE_APP_PORT`` / ``VITE_APP_BASE_API`` | 开发端口（5778）/ 接口前缀（``/api``，经 vite proxy 转发到后端） |',
    '| ``.env.production`` | ``VITE_APP_BASE_API`` | 生产接口前缀（``/prod-api``，由 nginx 反代到后端） |',
    '',
    '## 占位符清单',
    '',
    '| 占位符 | 含义 |',
    '|--------|------|',
    '| ``{{FRONTEND_TITLE}}`` | 前端标题（VITE_APP_TITLE） |',
    '| ``{{MODULE_PREFIX}}`` | 模块前缀（package.json name） |',
    '| ``{{API_BASE_URL_DEV}}`` | 开发环境后端地址（vite proxy：``/api``、``/v3/api-docs``、``/webjars`` 三段） |',
    '| ``{{COPYRIGHT_HOLDER}}`` | 版权方（页脚） |',
    '| ``{{COPYRIGHT_YEAR}}`` | 版权年份（页脚） |',
    '| ``{{PROJECT_NAME}}`` / ``{{SERVER_PORT}}`` | 其它通用占位 |',
    ''
)
Set-Utf8NoBomContent (Join-Path $Dest 'README.md') ($readmeLines -join "`n")

# 校验：占位符应已写入，本地联调地址不应残留在实际配置里
Write-Host "校验占位符..."
$vitePath = Join-Path $Dest 'vite.config.ts'
$checks = @(
    @{ File = '.env'; Needle = '{{FRONTEND_TITLE}}' },
    @{ File = 'vite.config.ts'; Needle = '{{API_BASE_URL_DEV}}' },
    @{ File = 'src\layouts\index.vue'; Needle = '{{COPYRIGHT_YEAR}}' },
    @{ File = 'src\layouts\index.vue'; Needle = '{{COPYRIGHT_HOLDER}}' },
    @{ File = 'package.json'; Needle = '{{MODULE_PREFIX}}-ui' }
)
foreach ($c in $checks) {
    $full = Join-Path $Dest $c.File
    $text = [System.IO.File]::ReadAllText($full)
    if (-not $text.Contains($c.Needle)) {
        throw "校验失败：$($c.File) 未包含 $($c.Needle)"
    }
}
if (([System.IO.File]::ReadAllText($vitePath)).Contains('localhost:14001')) {
    throw "校验失败：vite.config.ts 仍含本地联调地址 localhost:14001"
}

$fileCount = (Get-ChildItem $Dest -Recurse -File | Measure-Object).Count
Write-Host "完成：共 $fileCount 个文件 → $Dest"
