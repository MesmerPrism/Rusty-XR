<#
.SYNOPSIS
    Builds the public Rusty XR minimal Quest debug APK.
#>
[CmdletBinding()]
param(
    [string]$AndroidPlayerRoot = '',
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug',
    [switch]$SkipRustBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$exampleRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$repoRoot = (Resolve-Path (Join-Path $exampleRoot '..\..')).Path
$buildRoot = Join-Path $exampleRoot 'build'
$packageRoot = Join-Path $buildRoot 'package'
$classesRoot = Join-Path $buildRoot 'classes'
$dexRoot = Join-Path $buildRoot 'dex'
$outputsRoot = Join-Path $buildRoot 'outputs'
$keystoreRoot = Join-Path $buildRoot 'keystore'

function Invoke-Tool {
    param(
        [Parameter(Mandatory = $true)]
        [string]$File,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    Write-Host "> $File $($Arguments -join ' ')"
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code $LASTEXITCODE`: $File"
    }
}

function Find-AndroidPlayerRoot {
    param([string]$RequestedRoot)

    if (-not [string]::IsNullOrWhiteSpace($RequestedRoot)) {
        $resolved = (Resolve-Path $RequestedRoot).Path
        if ((Test-Path (Join-Path $resolved 'SDK')) -and
            (Test-Path (Join-Path $resolved 'NDK')) -and
            (Test-Path (Join-Path $resolved 'OpenJDK'))) {
            return $resolved
        }

        throw "AndroidPlayerRoot does not contain SDK, NDK, and OpenJDK: $resolved"
    }

    foreach ($envName in @('UNITY_ANDROID_PLAYER_ROOT', 'ANDROID_PLAYER_ROOT')) {
        $value = [Environment]::GetEnvironmentVariable($envName)
        if (-not [string]::IsNullOrWhiteSpace($value) -and (Test-Path $value)) {
            return Find-AndroidPlayerRoot -RequestedRoot $value
        }
    }

    $unityRoot = Join-Path $env:ProgramFiles 'Unity\Hub\Editor'
    if (Test-Path $unityRoot) {
        $candidate = Get-ChildItem -LiteralPath $unityRoot -Directory |
            ForEach-Object { Join-Path $_.FullName 'Editor\Data\PlaybackEngines\AndroidPlayer' } |
            Where-Object {
                (Test-Path (Join-Path $_ 'SDK')) -and
                (Test-Path (Join-Path $_ 'NDK')) -and
                (Test-Path (Join-Path $_ 'OpenJDK'))
            } |
            Sort-Object -Descending |
            Select-Object -First 1
        if (-not [string]::IsNullOrWhiteSpace($candidate)) {
            return $candidate
        }
    }

    throw 'Could not find Android tooling. Pass -AndroidPlayerRoot or set UNITY_ANDROID_PLAYER_ROOT.'
}

function Get-LatestDirectory {
    param(
        [string]$Parent,
        [string]$Pattern
    )

    $directory = Get-ChildItem -LiteralPath $Parent -Directory -Filter $Pattern |
        Sort-Object Name -Descending |
        Select-Object -First 1
    if ($null -eq $directory) {
        throw "No directory matching $Pattern under $Parent"
    }

    return $directory.FullName
}

function Reset-BuildDirectory {
    if (Test-Path $buildRoot) {
        $resolvedBuildRoot = (Resolve-Path $buildRoot).Path
        if (-not $resolvedBuildRoot.StartsWith($exampleRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to delete build directory outside the example root: $resolvedBuildRoot"
        }

        Remove-Item -LiteralPath $resolvedBuildRoot -Recurse -Force
    }

    New-Item -ItemType Directory -Force -Path $packageRoot, $classesRoot, $dexRoot, $outputsRoot, $keystoreRoot | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $packageRoot 'lib\arm64-v8a') | Out-Null
}

$androidRoot = Find-AndroidPlayerRoot -RequestedRoot $AndroidPlayerRoot
$sdkRoot = Join-Path $androidRoot 'SDK'
$ndkRoot = Join-Path $androidRoot 'NDK'
$jdkRoot = Join-Path $androidRoot 'OpenJDK'
$buildToolsRoot = Get-LatestDirectory -Parent (Join-Path $sdkRoot 'build-tools') -Pattern '*'
$platformRoot = Get-LatestDirectory -Parent (Join-Path $sdkRoot 'platforms') -Pattern 'android-*'
$androidJar = Join-Path $platformRoot 'android.jar'
$toolchainBin = Join-Path $ndkRoot 'toolchains\llvm\prebuilt\windows-x86_64\bin'
$linker = Join-Path $toolchainBin 'aarch64-linux-android29-clang.cmd'
if (-not (Test-Path $linker)) {
    $linker = Get-ChildItem -LiteralPath $toolchainBin -File -Filter 'aarch64-linux-android*-clang.cmd' |
        Sort-Object Name -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}

$javac = Join-Path $jdkRoot 'bin\javac.exe'
$jar = Join-Path $jdkRoot 'bin\jar.exe'
$keytool = Join-Path $jdkRoot 'bin\keytool.exe'
$aapt2 = Join-Path $buildToolsRoot 'aapt2.exe'
$d8 = Join-Path $buildToolsRoot 'd8.bat'
$zipalign = Join-Path $buildToolsRoot 'zipalign.exe'
$apksigner = Join-Path $buildToolsRoot 'apksigner.bat'

foreach ($tool in @($androidJar, $linker, $javac, $jar, $keytool, $aapt2, $d8, $zipalign, $apksigner)) {
    if (-not (Test-Path $tool)) {
        throw "Required Android build tool was not found: $tool"
    }
}

Reset-BuildDirectory

$env:ANDROID_HOME = $sdkRoot
$env:ANDROID_SDK_ROOT = $sdkRoot
$env:JAVA_HOME = $jdkRoot
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $linker
$llvmAr = Join-Path $toolchainBin 'llvm-ar.exe'
if (Test-Path $llvmAr) {
    $env:AR_aarch64_linux_android = $llvmAr
}

if (-not $SkipRustBuild) {
    Invoke-Tool -File 'cargo' -Arguments @(
        'build',
        '--manifest-path', (Join-Path $exampleRoot 'native\Cargo.toml'),
        '--target', 'aarch64-linux-android',
        '--release'
    )
}

$nativeLibrary = Join-Path $repoRoot 'target\aarch64-linux-android\release\librusty_xr_quest_minimal_native.so'
if (-not (Test-Path $nativeLibrary)) {
    throw "Rust native library was not found: $nativeLibrary"
}
Copy-Item -LiteralPath $nativeLibrary -Destination (Join-Path $packageRoot 'lib\arm64-v8a\librusty_xr_quest_minimal_native.so')

$unsignedApk = Join-Path $buildRoot 'rusty-xr-quest-minimal-unsigned.apk'
$alignedApk = Join-Path $buildRoot 'rusty-xr-quest-minimal-aligned.apk'
$outputApk = Join-Path $outputsRoot 'rusty-xr-quest-minimal-debug.apk'

Invoke-Tool -File $aapt2 -Arguments @(
    'link',
    '-o', $unsignedApk,
    '-I', $androidJar,
    '--manifest', (Join-Path $exampleRoot 'AndroidManifest.xml'),
    '--min-sdk-version', '29',
    '--target-sdk-version', '35'
)

$javaSources = Get-ChildItem -LiteralPath (Join-Path $exampleRoot 'src') -Recurse -File -Filter '*.java' |
    Select-Object -ExpandProperty FullName
$javacArgs = @(
    '-encoding', 'UTF-8',
    '-source', '1.8',
    '-target', '1.8',
    '-bootclasspath', $androidJar,
    '-d', $classesRoot
) + $javaSources
Invoke-Tool -File $javac -Arguments $javacArgs

$classFiles = Get-ChildItem -LiteralPath $classesRoot -Recurse -File -Filter '*.class' |
    Select-Object -ExpandProperty FullName
$d8Args = @(
    '--min-api', '29',
    '--output', $dexRoot
) + $classFiles
Invoke-Tool -File $d8 -Arguments $d8Args

Invoke-Tool -File $jar -Arguments @(
    'uf',
    $unsignedApk,
    '-C', $dexRoot, 'classes.dex',
    '-C', $packageRoot, 'lib'
)

Invoke-Tool -File $zipalign -Arguments @('-f', '-p', '4', $unsignedApk, $alignedApk)

$keystore = Join-Path $keystoreRoot 'rusty-xr-debug.keystore'
Invoke-Tool -File $keytool -Arguments @(
    '-genkeypair',
    '-keystore', $keystore,
    '-storepass', 'android',
    '-keypass', 'android',
    '-alias', 'androiddebugkey',
    '-keyalg', 'RSA',
    '-keysize', '2048',
    '-validity', '10000',
    '-dname', 'CN=Rusty XR Debug,O=Rusty XR,C=US',
    '-noprompt'
)

Invoke-Tool -File $apksigner -Arguments @(
    'sign',
    '--ks', $keystore,
    '--ks-pass', 'pass:android',
    '--key-pass', 'pass:android',
    '--out', $outputApk,
    $alignedApk
)
Invoke-Tool -File $apksigner -Arguments @('verify', '--print-certs', $outputApk)

Write-Host "Built APK: $outputApk"
