param(
    [Parameter(Mandatory = $true)]
    [string[]]$ArtifactDirs,
    [string]$OutDir = "",
    [string]$Python = "python"
)

$ErrorActionPreference = "Stop"

if (-not $OutDir) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutDir = Join-Path "artifacts\quest-streaming-diagnostics-scorecards" $timestamp
}

function Resolve-InputPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))
}

$resolvedArtifacts = @()
foreach ($dir in $ArtifactDirs) {
    $resolved = Resolve-InputPath -Path $dir
    if (-not (Test-Path -LiteralPath $resolved -PathType Container)) {
        throw "Artifact directory not found: $resolved"
    }
    $resolvedArtifacts += $resolved
}

$resolvedOutDir = Resolve-InputPath -Path $OutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

$parser = Join-Path $PSScriptRoot "Parse-QuestStreamingArtifact.py"
& $Python $parser @resolvedArtifacts `
    --json-out (Join-Path $resolvedOutDir "scorecard.json") `
    --markdown-out (Join-Path $resolvedOutDir "scorecard.md") |
    Tee-Object -FilePath (Join-Path $resolvedOutDir "scorecard-table.txt")

Write-Output $resolvedOutDir
