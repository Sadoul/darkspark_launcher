# Shared path constants for all engine-patches scripts.
# dot-source from other scripts: . $PSScriptRoot\_paths.ps1

$ErrorActionPreference = 'Stop'

$Global:RPWE_ROOT      = Split-Path -Parent $PSScriptRoot   # = engine-patches/
$Global:RPWE_ORIGINAL  = Join-Path $RPWE_ROOT 'original'
$Global:RPWE_PATCHES   = Join-Path $RPWE_ROOT 'patches'

$Global:MCP_REBORN     = 'C:\Users\smopo\Desktop\MinecraftEngine\references\MCP-Reborn-1.20'
$Global:MCP_SRC        = Join-Path $MCP_REBORN 'src\main\java'

$Global:RPWORLD        = 'C:\Users\smopo\AppData\Roaming\.rpworld\modpacks\rpworld'
$Global:RPWORLD_VER    = Join-Path $RPWORLD 'versions'

$Global:FORGE_MDK      = 'C:\Users\smopo\Desktop\Forge MDK 1.20.1 47.4.20'

# Map of "logical name" -> relative path inside MCP-Reborn src/main/java.
# Add new patched files here. apply.ps1 and restore.ps1 read this map.
$Global:RPWE_FILE_MAP = @{
    'PalettedContainer.java' = 'net\minecraft\world\level\chunk\PalettedContainer.java'
    'LevelChunkSection.java' = 'net\minecraft\world\level\chunk\LevelChunkSection.java'
    'MappedRegistry.java'    = 'net\minecraft\core\MappedRegistry.java'
    'ModelBakery.java'       = 'net\minecraft\client\resources\model\ModelBakery.java'
    'CompoundTag.java'       = 'net\minecraft\nbt\CompoundTag.java'
    'BlockBehaviour.java'     = 'net\minecraft\world\level\block\state\BlockBehaviour.java'
    'DataLayer.java'          = 'net\minecraft\world\level\chunk\DataLayer.java'
    'TextureAtlas.java'       = 'net\minecraft\client\renderer\texture\TextureAtlas.java'
    'SimpleReloadInstance.java' = 'net\minecraft\server\packs\resources\SimpleReloadInstance.java'
    'TextureAtlasSprite.java' = 'net\minecraft\client\renderer\texture\TextureAtlasSprite.java'
    'ModelBakeryExtreme.java' = 'net\minecraft\client\resources\model\ModelBakery.java'
    'ChunkMap.java'          = 'net\minecraft\server\level\ChunkMap.java'
    'RegionFile.java'         = 'net\minecraft\world\level\chunk\storage\RegionFile.java'
    'ClientResourcesDownloaded.java' = 'net\minecraft\client\resources\ClientResourcesDownloaded.java'
}

function Assert-Path([string]$p, [string]$label) {
    if (-not (Test-Path $p)) { throw "$label not found: $p" }
}
