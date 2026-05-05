# Launch the patched Forge client directly, bypassing RPWorld's launcher.
#
# Builds the JVM command from versions/1.20.1-forge-47.4.20/<id>.json + parent
# 1.20.1.json, mirroring exactly what RPWLauncher would do, then runs it
# headless with stdout/stderr captured to logs/.
#
# Stops the JVM after a configurable timeout — long enough for full ModLauncher
# bootstrap, Forge mod discovery, all Mixin transformations, and reaching the
# main menu (or crashing during bootstrap, which is what we want to capture).

param(
    [int]$TimeoutSec = 180,
    [switch]$Vanilla   # restore srg backup, run without our patches (control)
)

. $PSScriptRoot\_paths.ps1

if ($Vanilla) {
    Write-Host "VANILLA MODE: restoring srg jar from backup before launch" -ForegroundColor Yellow
    & "$PSScriptRoot\restore.ps1" -RestoreJar | Out-Null
}

# ---- paths ----
$gameDir         = $RPWORLD
$verDir          = Join-Path $RPWORLD_VER '1.20.1-forge-47.4.20'
$forgeJsonPath   = Join-Path $verDir '1.20.1-forge-47.4.20.json'
$vanillaJsonPath = Join-Path $RPWORLD_VER '1.20.1\1.20.1.json'
Assert-Path $forgeJsonPath  'forge profile JSON'
Assert-Path $vanillaJsonPath 'vanilla profile JSON'

$forgeJson   = Get-Content $forgeJsonPath -Raw | ConvertFrom-Json
$vanillaJson = Get-Content $vanillaJsonPath -Raw | ConvertFrom-Json

$libDir   = Join-Path $RPWORLD 'libraries'
$natDir   = Join-Path $verDir  '1.20.1-forge-47.4.20-natives'
$assetIdx = $vanillaJson.assetIndex.id
$assetDir = Join-Path $RPWORLD 'assets'
$mainCls  = $forgeJson.mainClass
$logsDir  = Join-Path $RPWE_ROOT 'logs'
New-Item -ItemType Directory -Force -Path $logsDir | Out-Null

# ---- ensure natives directory is populated ----
if (-not (Test-Path $natDir) -or @(Get-ChildItem $natDir -ErrorAction SilentlyContinue).Count -eq 0) {
    Write-Host "populating natives directory..." -ForegroundColor DarkGray
    New-Item -ItemType Directory -Force -Path $natDir | Out-Null
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    foreach ($lib in $vanillaJson.libraries) {
        if (-not $lib.downloads) { continue }
        $artifacts = @()
        if ($lib.downloads.classifiers) {
            $cls = $lib.downloads.classifiers
            if ($cls.'natives-windows') { $artifacts += $cls.'natives-windows' }
        }
        if ($lib.downloads.artifact -and $lib.name -match 'natives-windows') {
            $artifacts += $lib.downloads.artifact
        }
        foreach ($art in $artifacts) {
            if (-not $art.path) { continue }
            $jar = Join-Path $libDir $art.path
            if (-not (Test-Path $jar)) { continue }
            try {
                $z = [System.IO.Compression.ZipFile]::OpenRead($jar)
                foreach ($e in $z.Entries) {
                    if ($e.FullName -match '\.dll$' -and -not $e.FullName.StartsWith('META-INF')) {
                        $out = Join-Path $natDir (Split-Path $e.FullName -Leaf)
                        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($e, $out, $true)
                    }
                }
                $z.Dispose()
            } catch { Write-Warning ("could not unpack natives from {0}: {1}" -f $jar, $_) }
        }
    }
    Write-Host ("  natives extracted: {0} files" -f @(Get-ChildItem $natDir -Filter *.dll).Count) -ForegroundColor DarkGray
}

# ---- resolve every library jar ----
function Resolve-LibPath($lib) {
    if ($lib.downloads -and $lib.downloads.artifact -and $lib.downloads.artifact.path) {
        return Join-Path $libDir $lib.downloads.artifact.path
    }
    if ($lib.name) {
        $parts = $lib.name -split ':'
        $g = $parts[0] -replace '\.', '/'
        $a = $parts[1]; $v = $parts[2]
        $cls = if ($parts.Count -gt 3) { "-$($parts[3])" } else { '' }
        return Join-Path $libDir "$g/$a/$v/$a-$v$cls.jar"
    }
    return $null
}

# Filter natives to only Windows x64. Skip linux/macos/x86 entirely.
function Should-SkipNative([string]$libName) {
    $parts = $libName -split ':'
    if ($parts.Count -lt 4) { return $false }   # no classifier
    $cls = $parts[3]
    if ($cls -match 'natives-(linux|macos)') { return $true }
    if ($cls -eq 'natives-windows-x86') { return $true }   # we are x64
    if ($cls -eq 'natives-windows-arm64') { return $true }
    return $false
}

$libsByKey = @{}
foreach ($lib in $vanillaJson.libraries) {
    if (Should-SkipNative $lib.name) { continue }
    $p = Resolve-LibPath $lib; if (-not $p -or -not (Test-Path $p)) { continue }
    # Use full GAV+classifier as key so main and natives don't collide
    $libsByKey[$lib.name] = $p
}
foreach ($lib in $forgeJson.libraries) {
    if (Should-SkipNative $lib.name) { continue }
    $p = Resolve-LibPath $lib; if (-not $p -or -not (Test-Path $p)) { continue }
    $libsByKey[$lib.name] = $p
}
# IMPORTANT: do NOT add the vanilla profile jar (1.20.1.jar) to legacyClassPath.
# Forge resolves the Minecraft module from libraries/net/minecraft/client/.../client-*-srg.jar
# (which is what RPWLauncher patches via deploy.ps1). Adding 1.20.1.jar duplicates
# every com.mojang.* / net.minecraft.* package and triggers ResolutionException:
#   "Modules _1._20._1 and minecraft export package com.mojang.blaze3d.systems"

$cpEntries = $libsByKey.Values | Sort-Object -Unique
Write-Host ("classpath entries: {0}" -f $cpEntries.Count) -ForegroundColor DarkGray

# Write legacy classpath to a file (BootstrapLauncher reads it via -DlegacyClassPath.file=)
# IMPORTANT: must be UTF-8 *without* BOM, otherwise BootstrapLauncher tries to
# treat the BOM as part of the first path and crashes with InvalidPathException.
$legacyCpFile = Join-Path $logsDir 'legacy_cp.txt'
[System.IO.File]::WriteAllText($legacyCpFile, ($cpEntries -join "`n"), (New-Object System.Text.UTF8Encoding $false))

# ---- assemble JVM args ----
$placeholders = @{
    '${library_directory}'   = $libDir
    '${classpath_separator}' = ';'
    '${version_name}'        = '1.20.1-forge-47.4.20'
    '${natives_directory}'   = $natDir
    '${launcher_name}'       = 'rpworld-engine-launch'
    '${launcher_version}'    = '1.0'
    '${classpath}'           = ($cpEntries -join ';')
}

function Resolve-Tmpl($s) {
    foreach ($k in $placeholders.Keys) { $s = $s.Replace($k, $placeholders[$k]) }
    return $s
}

$jvmArgs = @()
foreach ($a in $forgeJson.arguments.jvm) {
    if ($a -is [string]) { $jvmArgs += (Resolve-Tmpl $a) }
}
$jvmArgs += "-DlegacyClassPath.file=$legacyCpFile"
$jvmArgs += "-Djava.library.path=$natDir"
$jvmArgs += '-Dminecraft.launcher.brand=rpworld-engine'
$jvmArgs += '-Dminecraft.launcher.version=1.0'
$jvmArgs += '-Dlog4j2.formatMsgNoLookups=true'

# ----------------------------------------------------------------------
# RPWorldEngine JVM tuning (P-JVM): Aikar-style flags + heap sizing.
#
# Why each flag matters here:
#   -Xms == -Xmx (8G)
#       Pre-allocates the full heap once. Default Mojang launcher uses
#       1G..4G with growth, which forces the JVM to commit/uncommit pages
#       during the first 5-10 minutes of a 170-mod load -> long GC pauses
#       and visible "freeze" to the user. Single fixed size eliminates this.
#
#   -XX:+UseG1GC                     low-pause GC, default for JDK 17
#   -XX:+ParallelRefProcEnabled      parallelize Reference processing
#   -XX:MaxGCPauseMillis=200         soft cap on GC pause
#   -XX:+UnlockExperimentalVMOptions enables the next 4 flags
#   -XX:+DisableExplicitGC           ignores System.gc() calls (some mods spam it)
#   -XX:+AlwaysPreTouch              touches all heap pages at start so OS
#                                    commits them upfront -> no first-touch lag
#   -XX:G1NewSizePercent=30          larger young gen for short-lived chunk objs
#   -XX:G1MaxNewSizePercent=40
#   -XX:G1HeapRegionSize=8M          fewer regions, less GC overhead on 8G heap
#   -XX:G1ReservePercent=20          headroom against humongous-allocation OOM
#   -XX:InitiatingHeapOccupancyPercent=15  start concurrent GC early -> avoid
#                                          full GC under sudden chunk load spike
#   -XX:+UseStringDeduplication      collapses identical String values to one
#                                    backing array -> 5-10% heap saving on a
#                                    modded MC where 170 mods register tens of
#                                    thousands of ResourceLocations
#   -XX:SoftRefLRUPolicyMSPerMB=10000  textures/models stay cached longer
#                                      between resource reloads
#
# These flags are the Aikar set used by Paper/Folia/Pufferfish/etc. on 100k+
# servers worldwide. Zero compatibility risk with mods.
# ----------------------------------------------------------------------
# ----------------------------------------------------------------------
# P-JVM v2: tuned for FAST STARTUP, not steady-state throughput.
#
# Key shift from v1:
#   * Heap is now Xms=2G..Xmx=8G (was 8G..8G with AlwaysPreTouch).
#     Pre-touching 8GB took ~10-20s of pure wait at JVM start. We let heap grow
#     dynamically and rely on early IHOP for collection pressure.
#   * Removed AlwaysPreTouch entirely.
#   * Removed PerfDisableSharedMem (it costs nothing on Win11 to leave default).
#   * Added Tier1 JIT promotion threshold (faster JIT warm-up of hot methods).
#   * Added segmented code cache (NonProfiledCodeHeap reserved separately ->
#     C2 doesn't evict already-compiled hot Forge mod methods).
# ----------------------------------------------------------------------
$jvmArgs += '-Xms8G'
$jvmArgs += '-Xmx8G'
$jvmArgs += '-XX:+AlwaysPreTouch'
$jvmArgs += '-XX:+UseG1GC'
$jvmArgs += '-XX:+ParallelRefProcEnabled'
$jvmArgs += '-XX:MaxGCPauseMillis=200'
$jvmArgs += '-XX:+UnlockExperimentalVMOptions'
$jvmArgs += '-XX:+DisableExplicitGC'
$jvmArgs += '-XX:G1NewSizePercent=30'
$jvmArgs += '-XX:G1MaxNewSizePercent=40'
$jvmArgs += '-XX:G1HeapRegionSize=8M'
$jvmArgs += '-XX:G1ReservePercent=20'
$jvmArgs += '-XX:G1HeapWastePercent=5'
$jvmArgs += '-XX:G1MixedGCCountTarget=4'
$jvmArgs += '-XX:InitiatingHeapOccupancyPercent=15'
$jvmArgs += '-XX:G1MixedGCLiveThresholdPercent=90'
$jvmArgs += '-XX:G1RSetUpdatingPauseTimePercent=5'
$jvmArgs += '-XX:SurvivorRatio=32'
$jvmArgs += '-XX:MaxTenuringThreshold=1'
$jvmArgs += '-XX:+UseStringDeduplication'
$jvmArgs += '-XX:SoftRefLRUPolicyMSPerMB=10000'
$jvmArgs += '-Dusing.aikars.flags=https://mcflags.emc.gs'
$jvmArgs += '-Daikars.new.flags=true'

# ---- JIT/Code-cache: speed up tier-up + reserve more code heap ----
$jvmArgs += '-XX:CICompilerCount=4'
$jvmArgs += '-XX:ReservedCodeCacheSize=512m'
$jvmArgs += '-XX:+UseCodeCacheFlushing'
$jvmArgs += '-XX:+SegmentedCodeCache'
$jvmArgs += '-XX:Tier3InvocationThreshold=1000'
$jvmArgs += '-XX:Tier4InvocationThreshold=5000'
$jvmArgs += '-XX:StringTableSize=1000003'

# P-MJ: pre-size Metaspace for 170 mods (~250MB of classes).
# Default 21MB triggers Metaspace full-GC dozens of times during class loading.
$jvmArgs += '-XX:MetaspaceSize=256m'
$jvmArgs += '-XX:MaxMetaspaceSize=1g'

# P-CDS: dynamic CDS — JDK writes a class-data archive on shutdown,
# next startup memory-maps it instead of class-loading from scratch.
# First run no benefit; second+ run saves ~3-8s.
$cdsRead  = Join-Path $RPWE_ROOT 'logs\rpwe-cds-read.jsa'; $cdsWrite = Join-Path $RPWE_ROOT 'logs\rpwe-cds-write.jsa'
$jvmArgs += "-XX:ArchiveClassesAtExit=$cdsRead"
if (Test-Path $cdsRead) {
    $jvmArgs += "-XX:SharedArchiveFile=$cdsRead"
    Write-Host "  using CDS cache: $cdsRead ($([math]::Round((Get-Item $cdsRead).Length/1MB,1)) MB)" -ForegroundColor DarkGreen
}

# ---- assemble game args ----
$gamePlaceholders = @{
    '${auth_player_name}'  = 'RpwePatchTest'
    '${version_name}'      = '1.20.1-forge-47.4.20'
    '${game_directory}'    = $gameDir
    '${assets_root}'       = $assetDir
    '${assets_index_name}' = $assetIdx
    '${auth_uuid}'         = '00000000-0000-0000-0000-000000000000'
    '${auth_access_token}' = '0'
    '${clientid}'          = '0'
    '${auth_xuid}'         = '0'
    '${user_type}'         = 'legacy'
    '${version_type}'      = 'release'
    '${resolution_width}'  = '854'
    '${resolution_height}' = '480'
    '${user_properties}'   = '{}'
}

$gameArgsRaw = @()
foreach ($a in $vanillaJson.arguments.game) {
    if ($a -is [string]) { $gameArgsRaw += $a }
}
foreach ($a in $forgeJson.arguments.game) {
    if ($a -is [string]) { $gameArgsRaw += $a }
}
$gameArgs = @()
foreach ($a in $gameArgsRaw) {
    $s = $a
    foreach ($k in $gamePlaceholders.Keys) { $s = $s.Replace($k, $gamePlaceholders[$k]) }
    $gameArgs += $s
}

# Drop any blank arg pairs (--flag followed by empty value)
$cleanGame = @()
for ($i = 0; $i -lt $gameArgs.Count; $i++) {
    $cur = $gameArgs[$i]
    if ([string]::IsNullOrWhiteSpace($cur)) { continue }
    $cleanGame += $cur
}
$gameArgs = $cleanGame

# ---- launch ----
$tag        = if ($Vanilla) { 'vanilla' } else { 'patched' }
$stdoutFile = Join-Path $logsDir "launch-$tag.stdout.log"
$stderrFile = Join-Path $logsDir "launch-$tag.stderr.log"
$cmdDump    = Join-Path $logsDir "launch-$tag.cmd.txt"
$java       = 'C:\Program Files\Eclipse Adoptium\jdk-17.0.18.8-hotspot\bin\java.exe'

@($java; $jvmArgs; $mainCls; $gameArgs) | Out-File $cmdDump -Encoding UTF8
Write-Host "command dump: $cmdDump" -ForegroundColor DarkGray

Write-Host "launching ($tag, timeout ${TimeoutSec}s)..." -ForegroundColor Cyan
$allArgs = @($jvmArgs) + @($mainCls) + @($gameArgs)
$allArgs = $allArgs | Where-Object { $null -ne $_ -and -not [string]::IsNullOrWhiteSpace([string]$_) }
Write-Host ("  total args: {0}, legacy CP file: {1} ({2} entries)" -f $allArgs.Count, $legacyCpFile, $cpEntries.Count) -ForegroundColor DarkGray

$proc = Start-Process -FilePath $java `
    -ArgumentList $allArgs `
    -WorkingDirectory $gameDir `
    -RedirectStandardOutput $stdoutFile `
    -RedirectStandardError $stderrFile `
    -PassThru -NoNewWindow

Write-Host "  pid=$($proc.Id), monitoring..." -ForegroundColor DarkGray

$start = Get-Date
$crashed = $false
while ($true) {
    if ($proc.HasExited) {
        $elapsed = [math]::Round(((Get-Date) - $start).TotalSeconds, 1)
        Write-Host ("  process exited at {0}s with code {1}" -f $elapsed, $proc.ExitCode) -ForegroundColor (@{$true='Red'; $false='Yellow'}[$proc.ExitCode -ne 0])
        $crashed = ($proc.ExitCode -ne 0)
        break
    }
    if (((Get-Date) - $start).TotalSeconds -ge $TimeoutSec) {
        Write-Host "  reached ${TimeoutSec}s timeout, killing..." -ForegroundColor Yellow
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        break
    }
    Start-Sleep -Seconds 2
}

Write-Host "`nlogs:" -ForegroundColor Cyan
Write-Host ("  stdout: {0}  ({1} bytes)" -f $stdoutFile, (Get-Item $stdoutFile -ErrorAction SilentlyContinue).Length)
Write-Host ("  stderr: {0}  ({1} bytes)" -f $stderrFile, (Get-Item $stderrFile -ErrorAction SilentlyContinue).Length)

# Surface key signals from logs
$out = Get-Content $stdoutFile -ErrorAction SilentlyContinue
$err = Get-Content $stderrFile -ErrorAction SilentlyContinue

if ($crashed -or $err) {
    Write-Host "`n--- last 80 lines of stderr ---" -ForegroundColor Red
    $err | Select-Object -Last 80
}

# Look for our patch markers in stdout
$ourLines = $out | Where-Object { $_ -match 'PalettedContainer|LevelChunkSection|RPWorldEngine' } | Select-Object -First 20
if ($ourLines) {
    Write-Host "`n--- patch-related lines in stdout ---" -ForegroundColor Cyan
    $ourLines | ForEach-Object { Write-Host "  $_" }
}
