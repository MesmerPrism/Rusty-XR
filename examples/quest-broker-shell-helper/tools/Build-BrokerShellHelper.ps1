<#
.SYNOPSIS
    Builds the public Rusty XR broker ADB shell-helper dex jar.
#>
[CmdletBinding()]
param(
    [string]$AndroidPlayerRoot = '',
    [string]$AndroidSdkRoot = '',
    [string]$JdkRoot = ''
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

    New-Item -ItemType Directory -Force -Path $classesRoot, $dexRoot, $outputsRoot | Out-Null
}

$toolchain = Resolve-RustyXrAndroidToolchain -AndroidPlayerRoot $AndroidPlayerRoot -AndroidSdkRoot $AndroidSdkRoot -JdkRoot $JdkRoot
$androidRoot = $toolchain.AndroidPlayerRoot
$sdkRoot = $toolchain.SdkRoot
$jdkRoot = $toolchain.JdkRoot
$buildToolsRoot = Get-RustyXrLatestAndroidDirectory -Parent (Join-Path $sdkRoot 'build-tools') -Pattern '*'
$platformRoot = Get-RustyXrLatestAndroidDirectory -Parent (Join-Path $sdkRoot 'platforms') -Pattern 'android-*'
$androidJar = Join-Path $platformRoot 'android.jar'
$javac = Join-Path $jdkRoot 'bin\javac.exe'
$jar = Join-Path $jdkRoot 'bin\jar.exe'
$d8 = Join-Path $buildToolsRoot 'd8.bat'

foreach ($tool in @($androidJar, $javac, $jar, $d8)) {
    if (-not (Test-Path $tool)) {
        throw "Required Android build tool was not found: $tool"
    }
}

Reset-BuildDirectory

$env:ANDROID_HOME = $sdkRoot
$env:ANDROID_SDK_ROOT = $sdkRoot
$env:JAVA_HOME = $jdkRoot

$javaSources = @(Get-ChildItem -LiteralPath (Join-Path $exampleRoot 'src') -Recurse -File -Filter '*.java' |
    Select-Object -ExpandProperty FullName)
if ($null -eq $javaSources -or $javaSources.Count -eq 0) {
    throw 'No Java sources found.'
}

Invoke-Tool -File $javac -Arguments (@(
    '-encoding', 'UTF-8',
    '-source', '1.8',
    '-target', '1.8',
    '-bootclasspath', $androidJar,
    '-d', $classesRoot
) + $javaSources)

$classFiles = @(Get-ChildItem -LiteralPath $classesRoot -Recurse -File -Filter '*.class' |
    Select-Object -ExpandProperty FullName)
Invoke-Tool -File $d8 -Arguments (@(
    '--min-api', '29',
    '--output', $dexRoot
) + $classFiles)

$outputJar = Join-Path $outputsRoot 'rusty-xr-broker-shell-helper.jar'
Invoke-Tool -File $jar -Arguments @(
    'cf',
    $outputJar,
    '-C', $dexRoot, 'classes.dex'
)

Write-Host "Built shell helper jar: $outputJar"
