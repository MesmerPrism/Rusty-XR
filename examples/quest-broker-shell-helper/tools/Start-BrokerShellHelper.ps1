<#
.SYNOPSIS
    Pushes and launches the Rusty XR broker ADB shell helper.
#>
[CmdletBinding()]
param(
    [string]$AndroidPlayerRoot = '',
    [string]$AndroidSdkRoot = '',
    [string]$JdkRoot = '',
    [string]$Adb = '',
    [string]$Serial = '',
    [string]$BrokerHost = '127.0.0.1',
    [int]$BrokerPort = 8765,
    [switch]$NoBuild,
    [switch]$Disconnect,
    [switch]$ProbeCodecs,
    [switch]$ProbeCameras,
    [switch]$ProbeCameraOpen,
    [string]$CameraOpenId = '',
    [switch]$EmitSyntheticVideoMetadata,
    [int]$SyntheticVideoSamples = 0,
    [switch]$EmitSyntheticVideoBinary,
    [int]$BinaryVideoPort = 8877,
    [int]$BinaryVideoPackets = 0,
    [int]$BinaryVideoPacketBytes = 0,
    [switch]$EmitMediaCodecSyntheticVideo,
    [switch]$EmitScreenrecordVideo,
    [int]$EncodedVideoFrames = 8,
    [int]$EncodedVideoWidth = 640,
    [int]$EncodedVideoHeight = 360,
    [int]$EncodedVideoBitrate = 1000000,
    [int]$ScreenrecordTimeLimit = 1,
    [switch]$ProximityWatchdog,
    [switch]$StopProximityWatchdog,
    [int]$ProximityWatchdogDurationMs = 28800000,
    [int]$ProximityWatchdogHoldDurationMs = 28800000,
    [int]$ProximityWatchdogIntervalMs = 5000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$exampleRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$repoRoot = (Resolve-Path (Join-Path $exampleRoot '..\..')).Path
. (Join-Path $repoRoot 'tools\android\Resolve-AndroidToolchain.ps1')
$helperJar = Join-Path $exampleRoot 'build\outputs\rusty-xr-broker-shell-helper.jar'
$deviceJar = '/data/local/tmp/rusty-xr-broker-shell-helper.jar'

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

function Resolve-Adb {
    param(
        [string]$RequestedAdb,
        [string]$RequestedAndroidRoot,
        [string]$RequestedAndroidSdkRoot,
        [string]$RequestedJdkRoot
    )

    if (-not [string]::IsNullOrWhiteSpace($RequestedAdb)) {
        return (Resolve-Path $RequestedAdb).Path
    }

    $toolchain = Resolve-RustyXrAndroidToolchain -AndroidPlayerRoot $RequestedAndroidRoot -AndroidSdkRoot $RequestedAndroidSdkRoot -JdkRoot $RequestedJdkRoot
    $candidate = Join-Path $toolchain.SdkRoot 'platform-tools\adb.exe'
    if (Test-Path $candidate) {
        return $candidate
    }

    throw "Could not find adb under Android SDK root: $($toolchain.SdkRoot)"
}

function Invoke-Adb {
    param([string[]]$Arguments)

    $prefix = @()
    if (-not [string]::IsNullOrWhiteSpace($Serial)) {
        $prefix = @('-s', $Serial)
    }
    Write-Host "> $script:adbPath $($prefix + $Arguments -join ' ')"
    & $script:adbPath @prefix @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "adb failed with exit code $LASTEXITCODE"
    }
}

if (-not $NoBuild) {
    & (Join-Path $PSScriptRoot 'Build-BrokerShellHelper.ps1') -AndroidPlayerRoot $AndroidPlayerRoot -AndroidSdkRoot $AndroidSdkRoot -JdkRoot $JdkRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Build-BrokerShellHelper.ps1 failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-Path $helperJar)) {
    throw "Shell helper jar was not found. Build it first: $helperJar"
}

$script:adbPath = Resolve-Adb -RequestedAdb $Adb -RequestedAndroidRoot $AndroidPlayerRoot -RequestedAndroidSdkRoot $AndroidSdkRoot -RequestedJdkRoot $JdkRoot

Invoke-Adb -Arguments @('push', $helperJar, $deviceJar)

$helperArgs = @(
    'shell',
    "CLASSPATH=$deviceJar",
    'app_process',
    '/',
    'com.example.rustyxr.shell.Helper',
    '--broker-host', $BrokerHost,
    '--broker-port', $BrokerPort.ToString()
)
if ($Disconnect) {
    $helperArgs += '--disconnect'
}
if ($ProbeCodecs) {
    $helperArgs += '--probe-codecs'
}
if ($ProbeCameras) {
    $helperArgs += '--probe-cameras'
}
if ($ProbeCameraOpen) {
    $helperArgs += '--probe-camera-open'
}
if (-not [string]::IsNullOrWhiteSpace($CameraOpenId)) {
    $helperArgs += @('--camera-open-id', $CameraOpenId)
}
if ($EmitSyntheticVideoMetadata) {
    $helperArgs += '--emit-synthetic-video-metadata'
}
if ($SyntheticVideoSamples -gt 0) {
    $helperArgs += @('--synthetic-video-samples', $SyntheticVideoSamples.ToString())
}
if ($EmitSyntheticVideoBinary) {
    $helperArgs += @('--emit-synthetic-video-binary', '--binary-video-port', $BinaryVideoPort.ToString())
}
if ($BinaryVideoPackets -gt 0) {
    $helperArgs += @('--binary-video-packets', $BinaryVideoPackets.ToString())
}
if ($BinaryVideoPacketBytes -gt 0) {
    $helperArgs += @('--binary-video-packet-bytes', $BinaryVideoPacketBytes.ToString())
}
if ($EmitMediaCodecSyntheticVideo) {
    $helperArgs += @(
        '--emit-mediacodec-synthetic-video',
        '--binary-video-port', $BinaryVideoPort.ToString(),
        '--encoded-video-frames', $EncodedVideoFrames.ToString(),
        '--encoded-video-width', $EncodedVideoWidth.ToString(),
        '--encoded-video-height', $EncodedVideoHeight.ToString(),
        '--encoded-video-bitrate', $EncodedVideoBitrate.ToString()
    )
}
if ($EmitScreenrecordVideo) {
    $helperArgs += @(
        '--emit-screenrecord-video',
        '--binary-video-port', $BinaryVideoPort.ToString(),
        '--encoded-video-width', $EncodedVideoWidth.ToString(),
        '--encoded-video-height', $EncodedVideoHeight.ToString(),
        '--encoded-video-bitrate', $EncodedVideoBitrate.ToString(),
        '--screenrecord-time-limit', $ScreenrecordTimeLimit.ToString()
    )
}
if ($ProximityWatchdog) {
    $helperArgs += @(
        '--proximity-watchdog',
        '--proximity-watchdog-duration-ms', $ProximityWatchdogDurationMs.ToString(),
        '--proximity-watchdog-hold-duration-ms', $ProximityWatchdogHoldDurationMs.ToString(),
        '--proximity-watchdog-interval-ms', $ProximityWatchdogIntervalMs.ToString()
    )
}
if ($StopProximityWatchdog) {
    $helperArgs += '--stop-proximity-watchdog'
}

if ($ProximityWatchdog -and -not $StopProximityWatchdog) {
    $deviceCommand = (($helperArgs | Select-Object -Skip 1) -join ' ') + ' > /data/local/tmp/rusty-xr-proximity-watchdog.log 2>&1 &'
    Invoke-Adb -Arguments @('shell', 'sh', '-c', $deviceCommand)
} else {
    Invoke-Adb -Arguments $helperArgs
}
