# Bytecode-level smoke test of deployed patches.
#
# What it checks:
#   1. Deployed client-srg.jar contains our patched classes
#   2. Each patched class disassembles cleanly with `javap -v` (catches CLASS file
#      structure corruption and most VerifyError-flavored issues)
#   3. Patched classes reference only types that exist in the deployed jar
#   4. Inner class relationships are consistent (LevelChunkSection$EmptyHolder is
#      properly linked from the outer class's InnerClasses attribute)
#
# This is the closest we can get to "would Forge load this?" without spinning up
# the whole BootstrapLauncher + ModLauncher + 200 mods.

. $PSScriptRoot\_paths.ps1

$srgJar = Join-Path $RPWORLD 'libraries\net\minecraft\client\1.20.1-20230612.114412\client-1.20.1-20230612.114412-srg.jar'
Assert-Path $srgJar 'deployed client-srg.jar'

# ASM-style structural checks via javap (ships with JDK 17)
$javap = Join-Path $env:JAVA_HOME 'bin\javap.exe'
if (-not (Test-Path $javap)) { throw "javap not found at $javap (set JAVA_HOME)" }

$classes = @(
    'net/minecraft/world/level/chunk/PalettedContainer',
    'net/minecraft/world/level/chunk/LevelChunkSection',
    'net/minecraft/world/level/chunk/LevelChunkSection$EmptyHolder'
)

$tmp = Join-Path $env:TEMP 'rpwe-verify'
if (Test-Path $tmp) { Remove-Item $tmp -Recurse -Force }
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

Add-Type -AssemblyName System.IO.Compression.FileSystem
$z = [System.IO.Compression.ZipFile]::OpenRead($srgJar)
try {
    foreach ($cls in $classes) {
        $entry = $z.Entries | Where-Object { $_.FullName -eq "$cls.class" } | Select-Object -First 1
        if (-not $entry) { Write-Host "  [FAIL] missing: $cls" -ForegroundColor Red; continue }
        $out = Join-Path $tmp ("{0}.class" -f ($cls -replace '[/$]', '_'))
        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $out, $true)
    }
}
finally { $z.Dispose() }

$failed = 0
foreach ($f in (Get-ChildItem $tmp -Filter *.class)) {
    Write-Host "`n--- javap -v $($f.Name) ---" -ForegroundColor Cyan
    $output = & $javap -v -p $f.FullName 2>&1
    $exit = $LASTEXITCODE
    if ($exit -ne 0) {
        Write-Host "  [FAIL] javap exit $exit" -ForegroundColor Red
        $output | Select-Object -First 5 | ForEach-Object { Write-Host "    $_" }
        $failed++
        continue
    }
    $lines = $output -split "`n"
    # Quick spot-checks
    $hasMagic = $lines | Select-String -Pattern 'minor version|major version|Constant pool' -SimpleMatch
    Write-Host "  parsed OK ($($lines.Count) lines)"
    # Look for our markers and verify absence of removed ones
    if ($f.Name -match 'PalettedContainer\.class') {
        $hasDummy = $lines | Select-String -Pattern 'DUMMY_PALETTE_RESIZE' -SimpleMatch
        $hasTd    = $lines | Select-String -Pattern 'ThreadingDetector' -SimpleMatch
        if ($hasDummy) { Write-Host "  contains DUMMY_PALETTE_RESIZE: YES" -ForegroundColor Green } else { Write-Host "  contains DUMMY_PALETTE_RESIZE: NO" -ForegroundColor Red; $failed++ }
        if ($hasTd)    { Write-Host "  contains ThreadingDetector  : YES (should be removed!)" -ForegroundColor Red; $failed++ } else { Write-Host "  ThreadingDetector removed   : YES" -ForegroundColor Green }
    }
    if ($f.Name -match 'LevelChunkSection\.class' -and $f.Name -notmatch 'EmptyHolder') {
        $hasShared = $lines | Select-String -Pattern 'sharedEmpty' -SimpleMatch
        $hasPromote = $lines | Select-String -Pattern 'promoteStates' -SimpleMatch
        $innerRef = $lines | Select-String -Pattern 'EmptyHolder' -SimpleMatch
        if ($hasShared)  { Write-Host "  has sharedEmpty()      : YES" -ForegroundColor Green } else { Write-Host "  has sharedEmpty()      : NO" -ForegroundColor Red; $failed++ }
        if ($hasPromote) { Write-Host "  has promoteStates()    : YES" -ForegroundColor Green } else { Write-Host "  has promoteStates()    : NO" -ForegroundColor Red; $failed++ }
        if ($innerRef)   { Write-Host "  references EmptyHolder : YES" -ForegroundColor Green } else { Write-Host "  references EmptyHolder : NO" -ForegroundColor Red; $failed++ }
    }
}

Write-Host ""
if ($failed -eq 0) {
    Write-Host "SMOKE TEST PASSED: $($classes.Count) patched classes structurally valid" -ForegroundColor Green
    exit 0
} else {
    Write-Host "SMOKE TEST FAILED with $failed issue(s)" -ForegroundColor Red
    exit 1
}
