<#
.SYNOPSIS
    Builds the public Rusty XR mic-pipe sentinel Quest debug APK.
#>
[CmdletBinding()]
param(
    [string]$AndroidPlayerRoot = '',
    [string]$AndroidSdkRoot = '',
    [string]$AndroidNdkRoot = '',
    [string]$JdkRoot = '',
    [ValidateRange(29, 35)]
    [int]$TargetSdkVersion = 35
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$exampleRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$repoRoot = (Resolve-Path (Join-Path $exampleRoot '..\..')).Path
. (Join-Path $repoRoot 'tools\android\Resolve-AndroidToolchain.ps1')

$buildRoot = Join-Path $exampleRoot 'build'
$classesRoot = Join-Path $buildRoot 'classes'
$dexRoot = Join-Path $buildRoot 'dex'
$outputsRoot = Join-Path $buildRoot 'outputs'
$debugKeyRoot = Join-Path $env:LOCALAPPDATA 'RustyXrDebugKeystores\quest-mic-pipe-sentinel'

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

function Reset-BuildDirectory {
    if (Test-Path $buildRoot) {
        $resolvedBuildRoot = (Resolve-Path $buildRoot).Path
        if (-not $resolvedBuildRoot.StartsWith($exampleRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to delete build directory outside the example root: $resolvedBuildRoot"
        }

        Remove-Item -LiteralPath $resolvedBuildRoot -Recurse -Force
    }

    New-Item -ItemType Directory -Force -Path $classesRoot, $dexRoot, $outputsRoot | Out-Null
}

$toolchain = Resolve-RustyXrAndroidToolchain `
    -AndroidPlayerRoot $AndroidPlayerRoot `
    -AndroidSdkRoot $AndroidSdkRoot `
    -AndroidNdkRoot $AndroidNdkRoot `
    -JdkRoot $JdkRoot
$sdkRoot = $toolchain.SdkRoot
$jdkRoot = $toolchain.JdkRoot
$buildToolsRoot = Get-RustyXrLatestAndroidDirectory -Parent (Join-Path $sdkRoot 'build-tools') -Pattern '*'
$platformRoot = Get-RustyXrLatestAndroidDirectory -Parent (Join-Path $sdkRoot 'platforms') -Pattern 'android-*'
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

$unsignedApk = Join-Path $buildRoot 'rusty-xr-quest-mic-pipe-sentinel-unsigned.apk'
$alignedApk = Join-Path $buildRoot 'rusty-xr-quest-mic-pipe-sentinel-aligned.apk'
$outputApk = Join-Path $outputsRoot 'rusty-xr-quest-mic-pipe-sentinel-debug.apk'
$classesJar = Join-Path $buildRoot 'rusty-xr-quest-mic-pipe-sentinel-classes.jar'

Invoke-Tool -File $aapt2 -Arguments @(
    'link',
    '-o', $unsignedApk,
    '-I', $androidJar,
    '--manifest', (Join-Path $exampleRoot 'AndroidManifest.xml'),
    '--min-sdk-version', '29',
    '--target-sdk-version', $TargetSdkVersion.ToString()
)

$javaSources = @(Get-ChildItem -LiteralPath (Join-Path $exampleRoot 'src') -Recurse -File -Filter '*.java' |
    Select-Object -ExpandProperty FullName)
if ($javaSources.Count -eq 0) {
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

Invoke-Tool -File $jar -Arguments @(
    'cf',
    $classesJar,
    '-C', $classesRoot, '.'
)
Invoke-Tool -File $d8 -Arguments @(
    '--min-api', '29',
    '--output', $dexRoot,
    $classesJar
)
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
