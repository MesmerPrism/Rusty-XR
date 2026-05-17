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
    [int]$ProximityWatchdogIntervalMs = 5000,
    [switch]$FocusGuardian,
    [switch]$StopFocusGuardian,
    [string]$FocusGuardianMode = 'observe',
    [string]$FocusGuardianDesiredFocus = 'broker',
    [string]$FocusTargetPackage = '',
    [string]$FocusTargetActivity = '',
    [string]$FocusBrokerPackage = 'com.example.rustyxr.broker',
    [string]$FocusBrokerActivity = 'com.example.rustyxr.broker.MainActivity',
    [int]$FocusGuardianDurationMs = 28800000,
    [int]$FocusGuardianIntervalMs = 1000,
    [int]$FocusGuardianCooldownMs = 1500
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$exampleRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$repoRoot = (Resolve-Path (Join-Path $exampleRoot '..\..')).Path
. (Join-Path $repoRoot 'tools\android\Resolve-AndroidToolchain.ps1')
$helperJar = Join-Path $exampleRoot 'build\outputs\rusty-xr-broker-shell-helper.jar'
$deviceJar = '/data/local/tmp/rusty-xr-broker-shell-helper.jar'

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

    $envAdb = [Environment]::GetEnvironmentVariable('RUSTY_XR_ADB')
    if (-not [string]::IsNullOrWhiteSpace($envAdb) -and (Test-Path -LiteralPath $envAdb)) {
        return (Resolve-Path -LiteralPath $envAdb).Path
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
if ($FocusGuardian) {
    $helperArgs += @(
        '--focus-guardian',
        '--focus-guardian-mode', $FocusGuardianMode,
        '--focus-guardian-desired-focus', $FocusGuardianDesiredFocus,
        '--focus-guardian-duration-ms', $FocusGuardianDurationMs.ToString(),
        '--focus-guardian-interval-ms', $FocusGuardianIntervalMs.ToString(),
        '--focus-guardian-cooldown-ms', $FocusGuardianCooldownMs.ToString()
    )
    if (-not [string]::IsNullOrWhiteSpace($FocusTargetPackage)) {
        $helperArgs += @('--focus-target-package', $FocusTargetPackage)
    }
    if (-not [string]::IsNullOrWhiteSpace($FocusTargetActivity)) {
        $helperArgs += @('--focus-target-activity', $FocusTargetActivity)
    }
    if (-not [string]::IsNullOrWhiteSpace($FocusBrokerPackage)) {
        $helperArgs += @('--focus-broker-package', $FocusBrokerPackage)
    }
    if (-not [string]::IsNullOrWhiteSpace($FocusBrokerActivity)) {
        $helperArgs += @('--focus-broker-activity', $FocusBrokerActivity)
    }
}
if ($StopFocusGuardian) {
    $helperArgs += '--stop-focus-guardian'
}

if (($ProximityWatchdog -or $FocusGuardian) -and -not $StopProximityWatchdog -and -not $StopFocusGuardian) {
    $logFile = if ($FocusGuardian) { '/data/local/tmp/rusty-xr-focus-guardian.log' } else { '/data/local/tmp/rusty-xr-proximity-watchdog.log' }
    $deviceCommand = (($helperArgs | Select-Object -Skip 1) -join ' ') + " > $logFile 2>&1 &"
    $singleQuote = [char]39
    $doubleQuote = [char]34
    $escapedDeviceCommand = $deviceCommand.Replace(
        "$singleQuote",
        "$singleQuote$doubleQuote$singleQuote$doubleQuote$singleQuote")
    $quotedDeviceCommand = "$singleQuote$escapedDeviceCommand$singleQuote"
    Invoke-Adb -Arguments @('shell', "sh -c $quotedDeviceCommand")
} else {
    Invoke-Adb -Arguments $helperArgs
}
