param(
    [string]$SdkPath,
    [string]$PackageName = "io.github.mesmerprism.rustyxr.makepad.camera",
    [string]$AppLabel = "Rusty XR Makepad Camera",
    [string]$CargoPackage = "rusty-xr-makepad-camera-shell",
    [ValidateSet("display-left-from-left-source", "display-left-from-right-source")]
    [string]$DisplaySourceEyeMapping = "display-left-from-left-source",
    [string]$WslDistro,
    [string]$MakepadSourceRoot,
    [switch]$PatchMakepadXrFromSource,
    [switch]$NoPatchMakepadXrFromSource,
    [switch]$UseWindowsHost
)

$ErrorActionPreference = "Stop"

$exampleRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

function Select-FirstNonEmpty {
    param([string[]]$Values)
    foreach ($value in $Values) {
        if (-not [string]::IsNullOrWhiteSpace($value)) {
            return $value
        }
    }
    return $null
}

function Get-HostExecutableName {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [ValidateSet("windows", "linux")][string]$HostKind
    )
    if ($HostKind -eq "windows") {
        return "$Name.exe"
    }
    return $Name
}

function Get-VersionSortKey {
    param([Parameter(Mandatory = $true)][string]$Name)
    $numbers = @([regex]::Matches($Name, "\d+") | ForEach-Object { [int64]$_.Value })
    while ($numbers.Count -lt 4) {
        $numbers += 0
    }
    return "{0:D8}.{1:D8}.{2:D8}.{3:D8}" -f $numbers[0], $numbers[1], $numbers[2], $numbers[3]
}

function Get-AndroidPlatformApi {
    param([Parameter(Mandatory = $true)][string]$PlatformName)
    $match = [regex]::Match($PlatformName, "^android-(\d+)")
    if (-not $match.Success) {
        return $null
    }
    return [int]$match.Groups[1].Value
}

function Resolve-AndroidPlatformName {
    param([Parameter(Mandatory = $true)][string]$SdkRoot)
    $requestedPlatform = Select-FirstNonEmpty @($env:ANDROID_PLATFORM)
    if ($requestedPlatform) {
        return $requestedPlatform
    }
    $requestedApi = Select-FirstNonEmpty @($env:ANDROID_SDK_VERSION)
    if ($requestedApi) {
        return "android-$requestedApi"
    }
    $platformsRoot = Join-Path $SdkRoot "platforms"
    $platform = Get-ChildItem -LiteralPath $platformsRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object { (Test-Path -LiteralPath (Join-Path $_.FullName "android.jar")) -and (Get-AndroidPlatformApi $_.Name) } |
        Sort-Object @{ Expression = { Get-AndroidPlatformApi $_.Name }; Descending = $true }, Name -Descending |
        Select-Object -First 1
    if (-not $platform) {
        throw "No installed Android platform with android.jar was found under $platformsRoot"
    }
    return $platform.Name
}

function Resolve-BuildToolsVersion {
    param([Parameter(Mandatory = $true)][string]$SdkRoot)
    $requested = Select-FirstNonEmpty @($env:ANDROID_BUILD_TOOLS_VERSION)
    if ($requested) {
        return $requested
    }
    $buildToolsRoot = Join-Path $SdkRoot "build-tools"
    $buildTools = Get-ChildItem -LiteralPath $buildToolsRoot -Directory -ErrorAction SilentlyContinue |
        Sort-Object @{ Expression = { Get-VersionSortKey $_.Name }; Descending = $true }, Name -Descending |
        Select-Object -First 1
    if (-not $buildTools) {
        throw "No installed Android build-tools directory was found under $buildToolsRoot"
    }
    return $buildTools.Name
}

function Resolve-JavaHome {
    param(
        [Parameter(Mandatory = $true)][string]$SdkRoot,
        [ValidateSet("windows", "linux")][string]$HostKind
    )
    $candidates = @()
    if (-not [string]::IsNullOrWhiteSpace($env:JAVA_HOME)) {
        $candidates += $env:JAVA_HOME
    }
    $candidates += (Join-Path $SdkRoot "openjdk")

    foreach ($candidate in $candidates) {
        $java = Join-Path $candidate ("bin\" + (Get-HostExecutableName -Name "java" -HostKind $HostKind))
        $javac = Join-Path $candidate ("bin\" + (Get-HostExecutableName -Name "javac" -HostKind $HostKind))
        if ((Test-Path -LiteralPath $java) -and (Test-Path -LiteralPath $javac)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    throw "No compatible Java home found for $HostKind. Set JAVA_HOME or use an SDK with an openjdk folder."
}

function Resolve-NdkPrebuiltRoot {
    param(
        [Parameter(Mandatory = $true)][string]$SdkRoot,
        [ValidateSet("windows", "linux")][string]$HostKind
    )
    $prebuiltName = if ($HostKind -eq "windows") { "windows-x86_64" } else { "linux-x86_64" }
    $ndkRoot = Join-Path $SdkRoot "ndk"
    $ndk = Get-ChildItem -LiteralPath $ndkRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName "toolchains\llvm\prebuilt\$prebuiltName") } |
        Sort-Object @{ Expression = { Get-VersionSortKey $_.Name }; Descending = $true }, Name -Descending |
        Select-Object -First 1
    if (-not $ndk) {
        throw "No compatible $prebuiltName NDK toolchain found under $ndkRoot. Use -UseWindowsHost with a Windows SDK or pass a Linux-host SDK for WSL builds."
    }
    return (Resolve-Path -LiteralPath (Join-Path $ndk.FullName "toolchains\llvm\prebuilt\$prebuiltName")).Path
}

function Resolve-ClangApiLevel {
    param(
        [Parameter(Mandatory = $true)][string]$NdkPrebuiltRoot,
        [ValidateSet("windows", "linux")][string]$HostKind,
        [int]$PlatformApi
    )
    $suffix = if ($HostKind -eq "windows") { "-clang.cmd" } else { "-clang" }
    $binRoot = Join-Path $NdkPrebuiltRoot "bin"
    $levels = Get-ChildItem -LiteralPath $binRoot -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match "^aarch64-linux-android(\d+)$([regex]::Escape($suffix))$" } |
        ForEach-Object { [int]([regex]::Match($_.Name, "^aarch64-linux-android(\d+)").Groups[1].Value) } |
        Sort-Object -Descending -Unique
    if (-not $levels) {
        throw "No aarch64-linux-android API-level clang wrapper found under $binRoot"
    }
    $requestedApi = Select-FirstNonEmpty @($env:ANDROID_API_LEVEL, $env:ANDROID_SDK_VERSION)
    if ($requestedApi) {
        $requestedApi = [int]$requestedApi
        if ($levels -contains $requestedApi) {
            return $requestedApi
        }
        throw "Requested Android API level $requestedApi has no clang wrapper under $binRoot; available levels: $($levels -join ', ')"
    }
    $best = $levels | Where-Object { $_ -le $PlatformApi } | Select-Object -First 1
    if ($best) {
        return $best
    }
    return ($levels | Select-Object -First 1)
}

function Assert-MakepadAndroidSdkProfile {
    param(
        [Parameter(Mandatory = $true)][string]$SdkPath,
        [ValidateSet("windows", "linux")][string]$HostKind
    )
    $sdkRoot = (Resolve-Path -LiteralPath $SdkPath).Path
    $platform = Resolve-AndroidPlatformName -SdkRoot $sdkRoot
    $platformApi = Get-AndroidPlatformApi $platform
    if (-not $platformApi) {
        throw "Selected Android platform '$platform' does not include an API level."
    }
    $androidJar = Join-Path $sdkRoot "platforms\$platform\android.jar"
    if (-not (Test-Path -LiteralPath $androidJar)) {
        throw "Selected Android platform '$platform' is missing android.jar at $androidJar"
    }

    $buildToolsVersion = Resolve-BuildToolsVersion -SdkRoot $sdkRoot
    $buildToolsRoot = Join-Path $sdkRoot "build-tools\$buildToolsVersion"
    foreach ($tool in @("aapt", "zipalign")) {
        $path = Join-Path $buildToolsRoot (Get-HostExecutableName -Name $tool -HostKind $HostKind)
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Selected build-tools '$buildToolsVersion' is missing $tool for $HostKind at $path"
        }
    }
    foreach ($jar in @("lib\d8.jar", "lib\apksigner.jar")) {
        $path = Join-Path $buildToolsRoot $jar
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Selected build-tools '$buildToolsVersion' is missing $jar at $path"
        }
    }

    $javaHome = Resolve-JavaHome -SdkRoot $sdkRoot -HostKind $HostKind
    $ndkPrebuiltRoot = Resolve-NdkPrebuiltRoot -SdkRoot $sdkRoot -HostKind $HostKind
    $compilerApi = Resolve-ClangApiLevel -NdkPrebuiltRoot $ndkPrebuiltRoot -HostKind $HostKind -PlatformApi $platformApi
    $clangName = "aarch64-linux-android$compilerApi-clang"
    if ($HostKind -eq "windows") {
        $clangName += ".cmd"
    }
    $clang = Join-Path $ndkPrebuiltRoot "bin\$clangName"
    if (-not (Test-Path -LiteralPath $clang)) {
        throw "Selected NDK prebuilt is missing $clangName at $clang"
    }

    [pscustomobject]@{
        SdkRoot = $sdkRoot
        HostKind = $HostKind
        Platform = $platform
        PlatformApi = $platformApi
        BuildToolsVersion = $buildToolsVersion
        JavaHome = $javaHome
        NdkPrebuiltRoot = $ndkPrebuiltRoot
        CompilerApi = $compilerApi
    }
}

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

function Resolve-DefaultCargoHome {
    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_HOME)) {
        return $env:CARGO_HOME
    }
    return Join-Path $HOME ".cargo"
}

function Add-CargoCacheLinks {
    param(
        [Parameter(Mandatory = $true)][string]$PatchCargoHome,
        [Parameter(Mandatory = $true)][string]$ExistingCargoHome
    )
    foreach ($cacheName in @("registry", "git")) {
        $source = Join-Path $ExistingCargoHome $cacheName
        $target = Join-Path $PatchCargoHome $cacheName
        if ((Test-Path -LiteralPath $source) -and -not (Test-Path -LiteralPath $target)) {
            try {
                New-Item -ItemType Junction -Path $target -Target $source -ErrorAction Stop | Out-Null
            } catch {
                Write-Warning "Unable to link Cargo $cacheName cache into temporary Makepad patch home: $($_.Exception.Message)"
            }
        }
    }
}

function New-MakepadPatchConfigText {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceRoot,
        [switch]$Wsl
    )

    $xrPath = Join-Path $SourceRoot "xr"
    if (-not (Test-Path $xrPath)) {
        throw "Makepad source root must contain an xr crate: $xrPath"
    }

    if ($Wsl) {
        $cargoPath = Convert-ToWslPath -Path $xrPath
    } else {
        $cargoPath = ((Resolve-Path $xrPath).Path) -replace '\\', '/'
    }

    return @"
[patch."https://github.com/MesmerPrism/makepad.git"]
makepad-xr = { path = "$cargoPath" }
"@
}

if ([string]::IsNullOrWhiteSpace($WslDistro)) {
    $WslDistro = Select-FirstNonEmpty @($env:MAKEPAD_WSL_DISTRO, "Ubuntu-Work")
}
if ([string]::IsNullOrWhiteSpace($MakepadSourceRoot)) {
    $MakepadSourceRoot = Select-FirstNonEmpty @($env:RUSTY_XR_MAKEPAD_SOURCE_ROOT)
}
if ($PatchMakepadXrFromSource -and $NoPatchMakepadXrFromSource) {
    throw "Use either -PatchMakepadXrFromSource or -NoPatchMakepadXrFromSource, not both."
}
if ($PatchMakepadXrFromSource -and [string]::IsNullOrWhiteSpace($MakepadSourceRoot)) {
    throw "-PatchMakepadXrFromSource requires -MakepadSourceRoot or RUSTY_XR_MAKEPAD_SOURCE_ROOT."
}
$patchMakepadXrFromSourceEffective = (-not [string]::IsNullOrWhiteSpace($MakepadSourceRoot)) -and (-not $NoPatchMakepadXrFromSource)

$hostKind = if ($UseWindowsHost) { "windows" } else { "linux" }
if ([string]::IsNullOrWhiteSpace($SdkPath)) {
    if ($UseWindowsHost) {
        $SdkPath = Select-FirstNonEmpty @($env:RUSTY_XR_ANDROID_SDK_ROOT, $env:ANDROID_SDK_ROOT, $env:ANDROID_HOME)
    } else {
        $SdkPath = Select-FirstNonEmpty @($env:MAKEPAD_ANDROID_SDK)
    }
    if ([string]::IsNullOrWhiteSpace($SdkPath)) {
        throw "No SDK path was provided. Activate the Android resolver profile or pass -SdkPath explicitly."
    }
}

$sdkProfile = Assert-MakepadAndroidSdkProfile -SdkPath $SdkPath -HostKind $hostKind
$SdkPath = $sdkProfile.SdkRoot
Write-Host ("Makepad Android SDK profile: host={0} sdk={1} platform={2} buildTools={3} compilerApi={4} java={5} ndkPrebuilt={6}" -f `
    $sdkProfile.HostKind, $sdkProfile.SdkRoot, $sdkProfile.Platform, $sdkProfile.BuildToolsVersion, $sdkProfile.CompilerApi, $sdkProfile.JavaHome, $sdkProfile.NdkPrebuiltRoot)
if ($MakepadSourceRoot) {
    Write-Host ("Makepad packager: source-built cargo-makepad from {0}; app dependency patching={1}" -f `
        (Resolve-Path -LiteralPath $MakepadSourceRoot).Path, $patchMakepadXrFromSourceEffective)
} else {
    Write-Host "Makepad packager: installed cargo-makepad from the active Cargo environment"
}

if ($UseWindowsHost) {
    Push-Location $exampleRoot
    $oldMapping = $env:RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING
    $oldCargoHome = $env:CARGO_HOME
    $oldJavaHome = $env:JAVA_HOME
    $oldAndroidHome = $env:ANDROID_HOME
    $oldAndroidSdkRoot = $env:ANDROID_SDK_ROOT
    $oldAndroidPlatform = $env:ANDROID_PLATFORM
    $oldAndroidSdkVersion = $env:ANDROID_SDK_VERSION
    $oldAndroidApiLevel = $env:ANDROID_API_LEVEL
    $oldAndroidBuildToolsVersion = $env:ANDROID_BUILD_TOOLS_VERSION
    $oldMakepadAndroidSdk = $env:MAKEPAD_ANDROID_SDK
    $patchCargoHome = $null
    $env:RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING = $DisplaySourceEyeMapping
    $env:JAVA_HOME = $sdkProfile.JavaHome
    $env:ANDROID_HOME = $sdkProfile.SdkRoot
    $env:ANDROID_SDK_ROOT = $sdkProfile.SdkRoot
    $env:ANDROID_PLATFORM = $sdkProfile.Platform
    $env:ANDROID_SDK_VERSION = [string]$sdkProfile.PlatformApi
    $env:ANDROID_API_LEVEL = [string]$sdkProfile.CompilerApi
    $env:ANDROID_BUILD_TOOLS_VERSION = $sdkProfile.BuildToolsVersion
    $env:MAKEPAD_ANDROID_SDK = $sdkProfile.SdkRoot
    try {
        if ($patchMakepadXrFromSourceEffective) {
            $patchCargoHome = Join-Path ([System.IO.Path]::GetTempPath()) "rusty-xr-makepad-cargo-$([guid]::NewGuid().ToString('N'))"
            New-Item -ItemType Directory -Force -Path $patchCargoHome | Out-Null
            Add-CargoCacheLinks -PatchCargoHome $patchCargoHome -ExistingCargoHome (Resolve-DefaultCargoHome)
            Set-Content -Path (Join-Path $patchCargoHome "config.toml") `
                -Value (New-MakepadPatchConfigText -SourceRoot $MakepadSourceRoot) `
                -Encoding UTF8
            $env:CARGO_HOME = $patchCargoHome
        }
        $cargoMakepadArgs = @()
        if ($MakepadSourceRoot) {
            $cargoMakepadArgs += @(
                "run",
                "--manifest-path",
                (Join-Path $MakepadSourceRoot "tools\cargo_makepad\Cargo.toml"),
                "--"
            )
        } else {
            $cargoMakepadArgs += "makepad"
        }
        $cargoMakepadArgs += @(
            "android",
            "--abi=aarch64",
            "--variant=quest",
            "--no-icon",
            "--sdk-path=$SdkPath",
            "--package-name=$PackageName",
            "--app-label=$AppLabel",
            "build"
        )
        $cargoMakepadArgs += @("-p", $CargoPackage, "--release")
        & cargo @cargoMakepadArgs
    } finally {
        if ($null -eq $oldMapping) {
            Remove-Item Env:\RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING -ErrorAction SilentlyContinue
        } else {
            $env:RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING = $oldMapping
        }
        if ($null -eq $oldCargoHome) {
            Remove-Item Env:\CARGO_HOME -ErrorAction SilentlyContinue
        } else {
            $env:CARGO_HOME = $oldCargoHome
        }
        foreach ($restore in @(
            @{ Name = "JAVA_HOME"; Value = $oldJavaHome },
            @{ Name = "ANDROID_HOME"; Value = $oldAndroidHome },
            @{ Name = "ANDROID_SDK_ROOT"; Value = $oldAndroidSdkRoot },
            @{ Name = "ANDROID_PLATFORM"; Value = $oldAndroidPlatform },
            @{ Name = "ANDROID_SDK_VERSION"; Value = $oldAndroidSdkVersion },
            @{ Name = "ANDROID_API_LEVEL"; Value = $oldAndroidApiLevel },
            @{ Name = "ANDROID_BUILD_TOOLS_VERSION"; Value = $oldAndroidBuildToolsVersion },
            @{ Name = "MAKEPAD_ANDROID_SDK"; Value = $oldMakepadAndroidSdk }
        )) {
            if ($null -eq $restore.Value) {
                Remove-Item "Env:\$($restore.Name)" -ErrorAction SilentlyContinue
            } else {
                Set-Item -Path "Env:\$($restore.Name)" -Value $restore.Value
            }
        }
        if ($patchCargoHome) {
            Remove-Item -LiteralPath $patchCargoHome -Recurse -Force -ErrorAction SilentlyContinue
        }
        Pop-Location
    }
    exit $LASTEXITCODE
}

$exampleRootWsl = Convert-ToWslPath -Path $exampleRoot
$sdkPathWsl = Convert-ToWslPath -Path $SdkPath
$javaHomeWsl = Convert-ToWslPath -Path $sdkProfile.JavaHome
$wslPatchCargoHome = $null
$wslPatchConfigBase64 = $null
if ($patchMakepadXrFromSourceEffective) {
    $wslPatchCargoHome = "/tmp/rusty-xr-makepad-cargo-$([guid]::NewGuid().ToString('N'))"
    $patchConfigText = New-MakepadPatchConfigText -SourceRoot $MakepadSourceRoot -Wsl
    $wslPatchConfigBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($patchConfigText))
}
$cargoCommandParts = @()
if ($MakepadSourceRoot) {
    $makepadToolManifestWsl = Convert-ToWslPath -Path (Join-Path $MakepadSourceRoot "tools\cargo_makepad\Cargo.toml")
    $cargoCommandParts += "cargo run --manifest-path=$(Quote-Bash $makepadToolManifestWsl) -- android"
} else {
    $cargoCommandParts += 'cargo makepad android'
}
$cargoCommandParts += @(
    '--abi=aarch64',
    '--variant=quest',
    '--no-icon',
    "--sdk-path=$(Quote-Bash $sdkPathWsl)",
    "--package-name=$(Quote-Bash $PackageName)",
    "--app-label=$(Quote-Bash $AppLabel)",
    'build'
)
$cargoCommandParts += @(
    '-p',
    (Quote-Bash $CargoPackage),
    '--release'
)
$cargoCommand = $cargoCommandParts -join ' '
$commandParts = @(
    'set -euo pipefail',
    'source "$HOME/.cargo/env"'
)
if ($wslPatchCargoHome) {
    $commandParts += @(
        "rm -rf $(Quote-Bash $wslPatchCargoHome)",
        "mkdir -p $(Quote-Bash $wslPatchCargoHome)",
        "trap $(Quote-Bash "rm -rf $(Quote-Bash $wslPatchCargoHome)") EXIT",
        "[ -d `"$HOME/.cargo/registry`" ] && ln -s `"$HOME/.cargo/registry`" $(Quote-Bash "$wslPatchCargoHome/registry") || true",
        "[ -d `"$HOME/.cargo/git`" ] && ln -s `"$HOME/.cargo/git`" $(Quote-Bash "$wslPatchCargoHome/git") || true",
        "printf %s $(Quote-Bash $wslPatchConfigBase64) | base64 -d > $(Quote-Bash "$wslPatchCargoHome/config.toml")",
        "export CARGO_HOME=$(Quote-Bash $wslPatchCargoHome)"
    )
}
$commandParts += @(
    "export JAVA_HOME=$(Quote-Bash $javaHomeWsl)",
    "export ANDROID_HOME=$(Quote-Bash $sdkPathWsl)",
    "export ANDROID_SDK_ROOT=$(Quote-Bash $sdkPathWsl)",
    "export MAKEPAD_ANDROID_SDK=$(Quote-Bash $sdkPathWsl)",
    "export ANDROID_PLATFORM=$(Quote-Bash $sdkProfile.Platform)",
    "export ANDROID_SDK_VERSION=$(Quote-Bash ([string]$sdkProfile.PlatformApi))",
    "export ANDROID_API_LEVEL=$(Quote-Bash ([string]$sdkProfile.CompilerApi))",
    "export ANDROID_BUILD_TOOLS_VERSION=$(Quote-Bash $sdkProfile.BuildToolsVersion)",
    "export RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING=$(Quote-Bash $DisplaySourceEyeMapping)",
    "cd $(Quote-Bash $exampleRootWsl)",
    $cargoCommand
)
$command = $commandParts -join '; '

& wsl.exe -d $WslDistro --exec /bin/bash -lc $command
if ($LASTEXITCODE -ne 0) {
    throw "WSL cargo makepad build failed with exit code $LASTEXITCODE"
}
