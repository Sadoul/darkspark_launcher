# release.ps1 - локальная сборка и публикация релиза DanganVerse Launcher

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$conf = Get-Content "src-tauri\tauri.conf.json" | ConvertFrom-Json
$VERSION = $conf.version
$TAG     = "v$VERSION"

# Автоинкремент patch если тег уже существует
while (git tag -l $TAG) {
    Write-Host "[INFO] Тег $TAG уже существует, авто-инкремент версии..." -ForegroundColor Yellow
    $parts = $VERSION -split '\.'
    $parts[-1] = [int]$parts[-1] + 1
    $VERSION = $parts -join '.'
    $TAG = "v$VERSION"

    # Обновляем версию в файлах
    $tauriConf = Get-Content "src-tauri\tauri.conf.json" -Raw
    $tauriConf = $tauriConf -replace '"version": "[^"]*"', "`"version`": `"$VERSION`""
    Set-Content "src-tauri\tauri.conf.json" $tauriConf -NoNewline

    (Get-Content "src-tauri\Cargo.toml") -replace '^version = ".*"', "version = `"$VERSION`"" |
        Set-Content "src-tauri\Cargo.toml"

    (Get-Content "stub-rs\Cargo.toml") -replace '^version = ".*"', "version = `"$VERSION`"" |
        Set-Content "stub-rs\Cargo.toml"

    Write-Host "[INFO] Версия обновлена до $VERSION" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  DanganVerse Launcher - Release $TAG" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "[1/4] Генерация иконок из images/icons/launcher.png..." -ForegroundColor Green
npx @tauri-apps/cli icon images/icons/launcher.png
if ($LASTEXITCODE -ne 0) { throw "Ошибка генерации иконок" }

Write-Host ""
Write-Host "[2/4] Сборка Tauri (NSIS installer)..." -ForegroundColor Green
npx tauri build --bundles nsis
if ($LASTEXITCODE -ne 0) { throw "Ошибка сборки Tauri" }

$nsisFiles = Get-ChildItem "src-tauri\target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "*$VERSION*" }
if (-not $nsisFiles) {
    $nsisFiles = Get-ChildItem "src-tauri\target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending
}
if (-not $nsisFiles) { throw "NSIS exe не найден" }
Write-Host "  -> Installer: $($nsisFiles[0].Name)" -ForegroundColor DarkGray

Write-Host ""
Write-Host "[3/4] Сборка DanganVerse-Launcher.exe (stub)..." -ForegroundColor Green
Push-Location stub-rs
cargo build --release
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "Ошибка сборки stub" }
Pop-Location

$stubExe = "stub-rs\target\release\DanganVerse-Launcher.exe"
if (-not (Test-Path $stubExe)) { throw "Stub exe не найден: $stubExe" }

Write-Host ""
Write-Host "[4/4] Публикация релиза $TAG на GitHub..." -ForegroundColor Green

$staged = git status --porcelain
if ($staged) {
    git add src-tauri/icons/ src-tauri/src/ src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json src/ stub-rs/ release.ps1
    git commit -m "Сохранение настроек игрока"
}

git tag $TAG
git push origin main
if ($LASTEXITCODE -ne 0) { throw "Ошибка git push (main)" }
git push origin "refs/tags/$TAG"
if ($LASTEXITCODE -ne 0) { throw "Ошибка git push (tag $TAG)" }

$releaseFiles = @($nsisFiles[0].FullName, (Resolve-Path $stubExe).Path)
gh release create $TAG `
    --title "DanganVerse Launcher $TAG" `
    --notes "Обновление лаунчера до версии $TAG" `
    @releaseFiles

if ($LASTEXITCODE -ne 0) { throw "Ошибка создания релиза" }

Write-Host ""
Write-Host "==================================================" -ForegroundColor Green
Write-Host "  Релиз $TAG успешно опубликован!" -ForegroundColor Green
Write-Host "  https://github.com/Sadoul/darkspark_launcher/releases/tag/$TAG" -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
