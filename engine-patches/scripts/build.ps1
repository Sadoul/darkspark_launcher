# Build patched Minecraft client jar via MCP-Reborn gradle.

. $PSScriptRoot\_paths.ps1

# Important: gradle prints javac deprecation warnings to stderr, which under
# $ErrorActionPreference='Stop' would otherwise abort the script. Override locally.
$ErrorActionPreference = 'Continue'

Assert-Path $MCP_REBORN 'MCP-Reborn'

Push-Location $MCP_REBORN
try {
    # We compile + reobf to SRG for Forge runtime. --rerun-tasks because Gradle's
    # incremental cache treats touched-via-filesystem .java overwrites as no-op.
    Write-Host "running: gradlew compileJava reobfJar --rerun-tasks" -ForegroundColor Cyan
    & .\gradlew.bat compileJava reobfJar --rerun-tasks --no-daemon --warning-mode=summary 2>&1 | ForEach-Object { "$_" }
    $exit = $LASTEXITCODE
    if ($exit -ne 0) {
        Write-Host "gradle exited with code $exit" -ForegroundColor Red
        throw "gradle build failed (code $exit)"
    }

    $reobfJar = Join-Path $MCP_REBORN 'build\reobfJar\output.jar'
    if (-not (Test-Path $reobfJar)) { throw "reobfJar output.jar not produced at $reobfJar" }

    Write-Host "`nartifact:" -ForegroundColor Green
    $info = Get-Item $reobfJar
    Write-Host ("  {0} ({1:N2} MB, modified {2})" -f $info.FullName, ($info.Length/1MB), $info.LastWriteTime)
}
finally { Pop-Location }
