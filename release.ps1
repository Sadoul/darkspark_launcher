# release.ps1 — локальная сборка и публикация релиза RPWorld Launcher
# Использование: .\release.ps1
# Требования: Rust, Node.js, gh CLI (winget install GitHub.cli)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Читаем версию из tauri.conf.json ──────────────────────────────────────────
$conf = Get-Content "src-tauri\tauri.conf.json" | ConvertFrom-Json
$VERSION = $conf.version
$TAG     = "v$VERSION"

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  RPWorld Launcher — Release $TAG" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# ── Проверяем, что тег ещё не существует ─────────────────────────────────────
$existingTag = git tag -l $TAG
if ($existingTag) {
    Write-Host "[WARN] Тег $TAG уже существует локально." -ForegroundColor Yellow
    $answer = Read-Host "Удалить его и пересоздать? (y/N)"
    if ($answer -ne "y") { exit 1 }
    git tag -d $TAG
}

# ── 1. Генерируем иконки ──────────────────────────────────────────────────────
Write-Host "[1/4] Генерация иконок из images/icons/launcher.png..." -ForegroundColor Green
npx @tauri-apps/cli icon images/icons/launcher.png
if ($LASTEXITCODE -ne 0) { throw "Ошибка генерации иконок" }

# ── 2. Сборка Tauri (NSIS installer) ─────────────────────────────────────────
Write-Host ""
Write-Host "[2/4] Сборка Tauri (NSIS installer)..." -ForegroundColor Green
npx tauri build --bundles nsis
if ($LASTEXITCODE -ne 0) { throw "Ошибка сборки Tauri" }

# Берём именно тот installer у которого в имени текущая версия
$nsisFiles = Get-ChildItem "src-tauri\target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "*$VERSION*" }
if (-not $nsisFiles) {
    # fallback — самый свежий файл
    $nsisFiles = Get-ChildItem "src-tauri\target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending
}
if (-not $nsisFiles) { throw "NSIS exe не найден в src-tauri\target\release\bundle\nsis\" }
Write-Host "  -> Installer: $($nsisFiles[0].Name)" -ForegroundColor DarkGray

# ── 3. Сборка stub exe ───────────────────────────────────────���────────────────
Write-Host ""
Write-Host "[3/4] Сборка RPWorld-Launcher.exe (stub)..." -ForegroundColor Green
Push-Location stub-rs
cargo build --release
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "Ошибка сборки stub" }
Pop-Location

$stubExe = "stub-rs\target\release\RPWorld-Launcher.exe"
if (-not (Test-Path $stubExe)) { throw "Stub exe не найден: $stubExe" }

# ── 4. Git tag + GitHub Release ───────────────────────────────────────────────
Write-Host ""
Write-Host "[4/4] Публикация релиза $TAG на GitHub..." -ForegroundColor Green

# Коммит всех изменений иконок если есть
$staged = git status --porcelain
if ($staged) {
    git add src-tauri/icons/ src-tauri/src/ src-tauri/Cargo.toml src-tauri/tauri.conf.json
    git commit -m "chore: release $TAG"
}

git tag $TAG
git push origin main --tags
if ($LASTEXITCODE -ne 0) { throw "Ошибка git push" }

# Создаём релиз и загружаем артефакты
$releaseFiles = @($nsisFiles[0].FullName, (Resolve-Path $stubExe).Path)
gh release create $TAG `
    --title "RPWorld Launcher $TAG" `
    --notes "Обновление лаунчера до версии $TAG" `
    @releaseFiles

if ($LASTEXITCODE -ne 0) { throw "Ошибка создания релиза" }

Write-Host ""
Write-Host "==================================================" -ForegroundColor Green
Write-Host "  Релиз $TAG успешно опубликован!" -ForegroundColor Green
Write-Host "  https://github.com/Sadoul/rpwlauncher/releases/tag/$TAG" -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
