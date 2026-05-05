# Restore vanilla decompiled files from ../original/ snapshots.
# With -RestoreJar: also restore .rpworld vanilla 1.20.1.jar from .rpwe-backup.

param(
    [switch]$RestoreJar
)

. $PSScriptRoot\_paths.ps1

Assert-Path $MCP_SRC      'MCP-Reborn src'
Assert-Path $RPWE_ORIGINAL 'engine-patches/original'

$restored = 0
foreach ($name in $RPWE_FILE_MAP.Keys) {
    $rel      = $RPWE_FILE_MAP[$name]
    $vanilla  = Join-Path $MCP_SRC $rel
    $snapshot = Join-Path $RPWE_ORIGINAL $name

    if (-not (Test-Path $snapshot)) { Write-Warning "no snapshot for $name (skipped)"; continue }

    Copy-Item $snapshot $vanilla -Force
    Write-Host "  restored $name" -ForegroundColor Yellow
    $restored++
}

Write-Host "`nRestored $restored source file(s)." -ForegroundColor Cyan

if ($RestoreJar) {
    # New target: forge's client-srg.jar in libraries/
    $mcLibDir  = Join-Path $RPWORLD 'libraries\net\minecraft\client\1.20.1-20230612.114412'
    $srgJar    = Join-Path $mcLibDir 'client-1.20.1-20230612.114412-srg.jar'
    $backupJar = "$srgJar.rpwe-backup"
    if (Test-Path $backupJar) {
        Copy-Item $backupJar $srgJar -Force
        Write-Host "restored Forge client-srg.jar from backup." -ForegroundColor Cyan
        $cache = "$srgJar.cache"; if (Test-Path $cache) { Remove-Item $cache -Force }
    } else {
        Write-Warning "no backup at $backupJar; srg jar not restored"
    }

    # Also restore the previously misused vanilla 1.20.1.jar if a backup exists
    $vJar = Join-Path $RPWORLD_VER '1.20.1\1.20.1.jar'
    $vBak = "$vJar.rpwe-backup"
    if (Test-Path $vBak) {
        Copy-Item $vBak $vJar -Force
        Write-Host "also restored .rpworld 1.20.1.jar (legacy backup)." -ForegroundColor Cyan
    }
}
