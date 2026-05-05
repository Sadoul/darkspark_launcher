# Deploy: inject reobfuscated patched classes directly into Forge's client-srg.jar.
#
# Forge 1.20.1 classpath layout (discovered from rpworld libraries):
#   libraries/net/minecraft/client/1.20.1-20230612.114412/
#     client-...-srg.jar     <- vanilla MC classes already SRG-renamed (m_NNN_, f_NNN_)
#     client-...-slim.jar    <- vanilla MC classes still in obf
#     client-...-extra.jar   <- assets/data/resources
#
# Forge runtime classloader resolves Minecraft classes from client-...-srg.jar.
# Since our reobfJar/output.jar is also SRG-named with the SAME class layout, we
# can safely overlay our patched classes on top of the vanilla SRG jar — Forge
# will see our versions instead of vanilla, and ALL Forge transformers/coremods
# continue to operate on top because the SRG signatures are unchanged.
#
# Safety:
#   - One-time backup: client-...-srg.jar.rpwe-backup
#   - Idempotent: each run resets to backup then re-overlays freshest reobf
#   - restore.ps1 -RestoreJar reverts everything
#
# This approach is invisible to the launcher (no profile change, no JSON edits) —
# the player simply launches the existing 1.20.1-forge-47.4.20 profile.

. $PSScriptRoot\_paths.ps1

$mcLibDir = Join-Path $RPWORLD 'libraries\net\minecraft\client\1.20.1-20230612.114412'
Assert-Path $mcLibDir 'minecraft client lib dir'

$srgJar    = Join-Path $mcLibDir 'client-1.20.1-20230612.114412-srg.jar'
$backupJar = "$srgJar.rpwe-backup"
$reobfJar  = Join-Path $MCP_REBORN 'build\reobfJar\output.jar'

Assert-Path $srgJar   'forge client-srg.jar'
Assert-Path $reobfJar 'reobfJar output (gradlew reobfJar)'

# Stage 1 — one-time backup of pristine srg jar
if (-not (Test-Path $backupJar)) {
    Copy-Item $srgJar $backupJar
    Write-Host "backup: $backupJar" -ForegroundColor DarkGray
} else {
    Write-Host "backup exists (preserved): $backupJar" -ForegroundColor DarkGray
}

# Stage 2 — reset target jar to pristine, then overlay
Copy-Item $backupJar $srgJar -Force

Add-Type -AssemblyName System.IO.Compression.FileSystem

# Overlay strategy: full overlay of all minecraft classes from our reobf jar.
#
# Why full and not selective: Forge mods (owo, embeddium etc.) build their
# Mixins against MCP-decompiled-then-recompiled bytecode shape, NOT against the
# vanilla forge-installer-produced client-srg.jar. The two have subtle LVT
# differences in some hot methods (e.g. OreFeature.m_225171_) that owo's
# Mixins do NOT tolerate. If we overlay only our 5 patched classes and leave
# OreFeature as the forge-shipped version, owo's Mixin LVT check fails.
#
# Our full reobf overlay produces a coherent bytecode set that mods compile
# against. Yes, this means classes we did not "patch" still get replaced with
# our recompiled versions — functionally identical, but structurally aligned
# with what mods expect.
#
# Known exclusions: classes where MCP-mapped name differs from forge-srg name
# (NoSuchMethodError'd in earlier tests). Listed in $EXCLUDED_PATHS.
$EXCLUDED_PATHS = @(
    # SinglePoolElement.<clinit> referenced m_210356_ in our reobf which doesn't
    # exist in forge-srg variant (it's been inlined or renamed).
    'net/minecraft/world/level/levelgen/structure/pools/SinglePoolElement.class'
)

$src = [System.IO.Compression.ZipFile]::OpenRead($reobfJar)
$dst = [System.IO.Compression.ZipFile]::Open($srgJar, [System.IO.Compression.ZipArchiveMode]::Update)
try {
    $overlaid = 0; $added = 0; $skipped = 0; $excluded = 0
    foreach ($entry in $src.Entries) {
        $name = $entry.FullName
        if ($name.EndsWith('/')) { continue }
        if (-not ($name.StartsWith('net/minecraft/') -or $name.StartsWith('com/mojang/'))) {
            $skipped++; continue
        }
        if ($EXCLUDED_PATHS -contains $name) {
            $excluded++; continue
        }

        $existing = $dst.GetEntry($name)
        if ($existing) { $existing.Delete(); $overlaid++ } else { $added++ }

        $newEntry  = $dst.CreateEntry($name, [System.IO.Compression.CompressionLevel]::Optimal)
        $srcStream = $entry.Open()
        $dstStream = $newEntry.Open()
        try { $srcStream.CopyTo($dstStream) } finally { $srcStream.Dispose(); $dstStream.Dispose() }
    }
    Write-Host ("overlaid {0}, added {1}, skipped {2} non-mc, excluded {3} known-divergent" -f $overlaid, $added, $skipped, $excluded) -ForegroundColor Green
}
finally {
    $dst.Dispose()
    $src.Dispose()
}

# Stage 3 — invalidate Forge's cache file (.cache holds sha1 of original srg jar)
$cacheFile = "$srgJar.cache"
if (Test-Path $cacheFile) {
    Remove-Item $cacheFile -Force
    Write-Host "removed stale cache: $($cacheFile | Split-Path -Leaf)" -ForegroundColor DarkGray
}

$newSize = [math]::Round((Get-Item $srgJar).Length/1MB, 2)
Write-Host ("`ndeployed. client-srg.jar is now {0} MB" -f $newSize) -ForegroundColor Green
Write-Host  "launch profile: 1.20.1-forge-47.4.20 (unchanged)" -ForegroundColor Cyan
Write-Host  "rollback: .\scripts\restore.ps1 -RestoreJar" -ForegroundColor DarkGray
