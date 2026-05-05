# Generate a standalone .bat launcher that runs RPWorld with our patches,
# bypassing RPWorld-Launcher.exe completely.
#
# Output: Desktop\RPWorld (engine).bat — double-click to play.

. $PSScriptRoot\_paths.ps1

$desktop = [Environment]::GetFolderPath('Desktop')
$bat = Join-Path $desktop 'RPWorld (engine).bat'
$ps1 = Join-Path $PSScriptRoot 'launch.ps1'

# Make launch.ps1 keep the window open instead of killing on timeout
$launchInteractive = Join-Path $PSScriptRoot 'launch-interactive.ps1'
Copy-Item $ps1 $launchInteractive -Force

# Tweak the copy: huge timeout (24h), don't kill, show stdout in real time
$content = Get-Content $launchInteractive -Raw
$content = $content -replace '\[int\]\$TimeoutSec = 180', '[int]$TimeoutSec = 86400'
# We want the JVM stdout to go to the console as well, so user sees progress
$content = $content -replace '-RedirectStandardOutput \$stdoutFile `\r?\n    -RedirectStandardError \$stderrFile `\r?\n    -PassThru -NoNewWindow', '-PassThru -NoNewWindow -Wait'
Set-Content $launchInteractive $content -Encoding UTF8

# .bat just calls PowerShell with bypass to run our script
$batContent = @"
@echo off
title RPWorld (engine patches)
echo.
echo === RPWorld with engine patches ===
echo Loading 170 mods, this can take 5-10 minutes on first launch.
echo Window will close when game exits.
echo.
powershell -ExecutionPolicy Bypass -NoProfile -File "$launchInteractive"
echo.
echo === game exited ===
pause
"@

Set-Content $bat $batContent -Encoding ASCII

Write-Host "shortcut created: $bat" -ForegroundColor Green
Write-Host "double-click to launch RPWorld with patches." -ForegroundColor Cyan
