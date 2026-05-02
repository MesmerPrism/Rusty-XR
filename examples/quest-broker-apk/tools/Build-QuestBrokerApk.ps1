<#
.SYNOPSIS
    Builds the public Rusty XR Quest broker proof-of-concept debug APK.
#>
[CmdletBinding()]
param(
    [string]$AndroidPlayerRoot = '',
    [string]$LslAndroidLibraryPath = '',
    [ValidateRange(29, 35)]
    [int]$TargetSdkVersion = 35
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$exampleRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$buildRoot = Join-Path $exampleRoot 'build'
$classesRoot = Join-Path $buildRoot 'classes'
$dexRoot = Join-Path $buildRoot 'dex'
$nativeRoot = Join-Path $buildRoot 'native'
$packageRoot = Join-Path $buildRoot 'package'
$outputsRoot = Join-Path $buildRoot 'outputs'
$debugKeyRoot = Join-Path $env:LOCALAPPDATA 'RustyXrDebugKeystores\quest-broker'

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
            (Test-Path (Join-Path $resolved 'OpenJDK'))) {
            return $resolved
        }

        throw "AndroidPlayerRoot does not contain SDK and OpenJDK: $resolved"
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
                (Test-Path (Join-Path $_ 'OpenJDK'))
            } |
            Sort-Object -Descending |
            Select-Object -First 1
        if ($null -ne $candidate) {
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

    New-Item -ItemType Directory -Force -Path $classesRoot, $dexRoot, $nativeRoot, $packageRoot, $outputsRoot | Out-Null
}

function Resolve-LslAndroidLibrary {
    param([string]$RequestedPath)

    if (-not [string]::IsNullOrWhiteSpace($RequestedPath)) {
        $resolved = (Resolve-Path $RequestedPath).Path
        if ((Split-Path -Leaf $resolved) -ne 'liblsl.so') {
            throw "LslAndroidLibraryPath must point to liblsl.so: $resolved"
        }

        return $resolved
    }

    $envPath = [Environment]::GetEnvironmentVariable('RUSTY_XR_ANDROID_LIBLSL')
    if (-not [string]::IsNullOrWhiteSpace($envPath)) {
        return Resolve-LslAndroidLibrary -RequestedPath $envPath
    }

    return ''
}

function Resolve-NdkToolchainRoot {
    param([string]$AndroidRoot)

    $candidate = Join-Path $AndroidRoot 'NDK\toolchains\llvm\prebuilt\windows-x86_64'
    if (Test-Path $candidate) {
        return $candidate
    }

    throw 'Native LSL packaging requires the Android NDK from the Android player root.'
}

$androidRoot = Find-AndroidPlayerRoot -RequestedRoot $AndroidPlayerRoot
$sdkRoot = Join-Path $androidRoot 'SDK'
$jdkRoot = Join-Path $androidRoot 'OpenJDK'
$buildToolsRoot = Get-LatestDirectory -Parent (Join-Path $sdkRoot 'build-tools') -Pattern '*'
$platformRoot = Get-LatestDirectory -Parent (Join-Path $sdkRoot 'platforms') -Pattern 'android-*'
$androidJar = Join-Path $platformRoot 'android.jar'
$javac = Join-Path $jdkRoot 'bin\javac.exe'
$jar = Join-Path $jdkRoot 'bin\jar.exe'
$keytool = Join-Path $jdkRoot 'bin\keytool.exe'
$aapt2 = Join-Path $buildToolsRoot 'aapt2.exe'
$d8 = Join-Path $buildToolsRoot 'd8.bat'
$zipalign = Join-Path $buildToolsRoot 'zipalign.exe'
$apksigner = Join-Path $buildToolsRoot 'apksigner.bat'

foreach ($tool in @($androidJar, $javac, $jar, $keytool, $aapt2, $d8, $zipalign, $apksigner)) {
    if (-not (Test-Path $tool)) {
        throw "Required Android build tool was not found: $tool"
    }
}

Reset-BuildDirectory

$env:ANDROID_HOME = $sdkRoot
$env:ANDROID_SDK_ROOT = $sdkRoot
$env:JAVA_HOME = $jdkRoot

$unsignedApk = Join-Path $buildRoot 'rusty-xr-quest-broker-unsigned.apk'
$alignedApk = Join-Path $buildRoot 'rusty-xr-quest-broker-aligned.apk'
$outputApk = Join-Path $outputsRoot 'rusty-xr-quest-broker-debug.apk'
$jniLibraryRoot = Join-Path $nativeRoot 'arm64-v8a'
$packagedLibraryRoot = Join-Path $packageRoot 'lib\arm64-v8a'

Invoke-Tool -File $aapt2 -Arguments @(
    'link',
    '-o', $unsignedApk,
    '-I', $androidJar,
    '--manifest', (Join-Path $exampleRoot 'AndroidManifest.xml'),
    '--min-sdk-version', '29',
    '--target-sdk-version', $TargetSdkVersion.ToString()
)

$javaSources = Get-ChildItem -LiteralPath (Join-Path $exampleRoot 'src') -Recurse -File -Filter '*.java' |
    Select-Object -ExpandProperty FullName
if ($null -eq $javaSources -or $javaSources.Count -eq 0) {
    throw 'No Java sources found.'
}

$javacArgs = @(
    '-encoding', 'UTF-8',
    '-source', '1.8',
    '-target', '1.8',
    '-bootclasspath', $androidJar,
    '-d', $classesRoot
) + $javaSources
Invoke-Tool -File $javac -Arguments $javacArgs

$lslAndroidLibrary = Resolve-LslAndroidLibrary -RequestedPath $LslAndroidLibraryPath
if (-not [string]::IsNullOrWhiteSpace($lslAndroidLibrary)) {
    $ndkToolchainRoot = Resolve-NdkToolchainRoot -AndroidRoot $androidRoot
    $clangxx = Join-Path $ndkToolchainRoot 'bin\aarch64-linux-android29-clang++.cmd'
    $libcxxShared = Join-Path $ndkToolchainRoot 'sysroot\usr\lib\aarch64-linux-android\libc++_shared.so'
    foreach ($tool in @($clangxx, $libcxxShared)) {
        if (-not (Test-Path $tool)) {
            throw "Required native LSL build input was not found: $tool"
        }
    }

    New-Item -ItemType Directory -Force -Path $jniLibraryRoot, $packagedLibraryRoot | Out-Null
    Copy-Item -LiteralPath $lslAndroidLibrary -Destination (Join-Path $jniLibraryRoot 'liblsl.so') -Force

    Invoke-Tool -File $clangxx -Arguments @(
        '-shared',
        '-fPIC',
        '-O2',
        '-Wall',
        '-Wextra',
        '-o', (Join-Path $jniLibraryRoot 'librustyxr_broker_lsl_jni.so'),
        (Join-Path $exampleRoot 'jni\rustyxr_broker_lsl_jni.cpp'),
        '-L', $jniLibraryRoot,
        '-llsl'
    )

    Copy-Item -LiteralPath (Join-Path $jniLibraryRoot 'liblsl.so') -Destination (Join-Path $packagedLibraryRoot 'liblsl.so') -Force
    Copy-Item -LiteralPath (Join-Path $jniLibraryRoot 'librustyxr_broker_lsl_jni.so') -Destination (Join-Path $packagedLibraryRoot 'librustyxr_broker_lsl_jni.so') -Force
    Copy-Item -LiteralPath $libcxxShared -Destination (Join-Path $packagedLibraryRoot 'libc++_shared.so') -Force
    Invoke-Tool -File $jar -Arguments @(
        'uf',
        $unsignedApk,
        '-C', $packageRoot, 'lib'
    )
} else {
    Write-Warning 'No Android liblsl.so supplied. Broker APK will accept samples, publish OSC when enabled, and log LSL fallback diagnostics.'
}

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
    '-C', $dexRoot, 'classes.dex'
)

Invoke-Tool -File $zipalign -Arguments @('-f', '-p', '4', $unsignedApk, $alignedApk)

$keystore = Join-Path $debugKeyRoot 'rusty-xr-debug.keystore'
New-Item -ItemType Directory -Force -Path $debugKeyRoot | Out-Null
if (-not (Test-Path $keystore)) {
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
}

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
