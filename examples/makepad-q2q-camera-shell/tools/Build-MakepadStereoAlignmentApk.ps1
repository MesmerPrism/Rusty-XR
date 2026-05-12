param(
    [Parameter(Mandatory = $true)]
    [string]$SdkPath,
    [string]$PackageName = "com.example.rustyxr.makepad.alignment",
    [string]$AppLabel = "Rusty XR Makepad Alignment",
    [string]$CargoPackage = "rusty-xr-makepad-q2q-camera-shell",
    [ValidateSet("display-left-from-left-source", "display-left-from-right-source")]
    [string]$DisplaySourceEyeMapping = "display-left-from-left-source",
    [string]$WslDistro = "Ubuntu",
    [switch]$UseWindowsHost
)

$ErrorActionPreference = "Stop"

$exampleRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

function Convert-ToWslPath {
    param([Parameter(Mandatory = $true)][string]$Path)
    $resolved = (Resolve-Path $Path).Path
    if ($resolved -match '^([A-Za-z]):\\(.*)$') {
        $drive = $matches[1].ToLowerInvariant()
        $tail = $matches[2] -replace '\\', '/'
        return "/mnt/$drive/$tail"
    }
    return $resolved
}

function Quote-Bash {
    param([Parameter(Mandatory = $true)][string]$Text)
    return "'" + ($Text -replace "'", "'\''") + "'"
}

if ($UseWindowsHost) {
    Push-Location $exampleRoot
    $oldMapping = $env:RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING
    $env:RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING = $DisplaySourceEyeMapping
    try {
        cargo makepad android `
            --abi=aarch64 `
            --variant=quest `
            --no-icon `
            --sdk-path=$SdkPath `
            --package-name=$PackageName `
            --app-label=$AppLabel `
            build `
            -p $CargoPackage `
            --release
    } finally {
        if ($null -eq $oldMapping) {
            Remove-Item Env:\RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING -ErrorAction SilentlyContinue
        } else {
            $env:RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING = $oldMapping
        }
        Pop-Location
    }
    exit $LASTEXITCODE
}

$exampleRootWsl = Convert-ToWslPath -Path $exampleRoot
$sdkPathWsl = Convert-ToWslPath -Path $SdkPath
$cargoCommand = @(
    'cargo makepad android',
    '--abi=aarch64',
    '--variant=quest',
    '--no-icon',
    "--sdk-path=$(Quote-Bash $sdkPathWsl)",
    "--package-name=$(Quote-Bash $PackageName)",
    "--app-label=$(Quote-Bash $AppLabel)",
    'build',
    '-p',
    (Quote-Bash $CargoPackage),
    '--release'
) -join ' '
$command = @(
    'set -euo pipefail',
    'source "$HOME/.cargo/env"',
    "export RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING=$(Quote-Bash $DisplaySourceEyeMapping)",
    "cd $(Quote-Bash $exampleRootWsl)",
    $cargoCommand
) -join '; '

& wsl.exe -d $WslDistro --exec /bin/bash -lc $command
if ($LASTEXITCODE -ne 0) {
    throw "WSL cargo makepad build failed with exit code $LASTEXITCODE"
}
