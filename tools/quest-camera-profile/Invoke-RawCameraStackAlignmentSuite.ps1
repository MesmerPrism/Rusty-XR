[CmdletBinding(PositionalBinding = $false)]
param(
    [string]$Serial = "",
    [string]$Adb = "adb",
    [string]$CompositeApk = "",
    [string]$GlesApk = "",
    [string]$MakepadApk = "",
    [string]$MakepadPackageName = "",
    [string]$MakepadLauncherActivity = "",
    [string]$MakepadXrActivity = "",
    [string]$RunRoot = "artifacts\raw-stack-suite",
    [string[]]$Mode = @(),
    [switch]$Install,
    [int]$WarmupSeconds = 20,
    [int]$SampleSeconds = 20,
    [int]$FreshnessFrames = 6,
    [int]$FreshnessIntervalMs = 1000,
    [ValidateSet("broker-camera", "broker-synthetic")]
    [string]$BrokerH264SourceMode = "broker-camera",
    [string]$BrokerH264LeftCameraId = "50",
    [string]$BrokerH264RightCameraId = "51",
    [string]$BrokerH264SyntheticPattern = "diagnostic-grid",
    [ValidateSet("head-anchored-virtual-camera", "camera-matched", "full-frame-diagnostic")]
    [string]$BrokerH264SyntheticProjectionProfile = "head-anchored-virtual-camera",
    [int]$BrokerH264LeftStreamPort = 8879,
    [int]$BrokerH264RightStreamPort = 8880,
    [int]$BrokerH264Width = 1280,
    [int]$BrokerH264Height = 1280,
    [int]$BrokerH264CaptureMs = 0,
    [int]$BrokerH264MaxPackets = 0,
    [int]$BrokerH264BitrateBps = 6000000,
    [int]$BrokerH264FrameRateHz = 50,
    [switch]$RestartBrokerBeforeBrokerModes,
    [string]$BrokerPackageName = "com.example.rustyxr.broker",
    [string]$BrokerActivityName = ".MainActivity",
    [int]$BrokerRestartSettleSeconds = 8,
    [ValidateSet("solid-red", "diagnostic-split", "passthrough-underlay")]
    [string]$ProjectionBorderPolicy = "solid-red",
    [ValidateSet("raw", "blur")]
    [string]$ProcessingLayer = "raw",
    [double]$BlurRadiusPx = 2.0,
    [double]$ProjectionAreaOffsetYUv = 0.0,
    [double]$VulkanProjectionAreaOffsetYUv = [double]::NaN,
    [double]$GlesProjectionAreaOffsetYUv = [double]::NaN,
    [double]$MakepadProjectionAreaOffsetYUv = [double]::NaN,
    [double]$VulkanDirectProjectionAreaOffsetYUv = [double]::NaN,
    [double]$VulkanBrokerProjectionAreaOffsetYUv = [double]::NaN,
    [double]$GlesDirectProjectionAreaOffsetYUv = [double]::NaN,
    [double]$GlesBrokerProjectionAreaOffsetYUv = [double]::NaN,
    [double]$MakepadDirectProjectionAreaOffsetYUv = [double]::NaN,
    [double]$MakepadBrokerProjectionAreaOffsetYUv = [double]::NaN,
    [double]$FullFrameBrokerProjectionAreaOffsetYUv = [double]::NaN,
    [double]$VulkanFullFrameBrokerProjectionAreaOffsetYUv = [double]::NaN,
    [double]$GlesFullFrameBrokerProjectionAreaOffsetYUv = [double]::NaN,
    [double]$MakepadFullFrameBrokerProjectionAreaOffsetYUv = [double]::NaN,
    [double]$ProjectionAreaScaleUv = 1.0,
    [double]$GlesProjectionAreaScaleUv = [double]::NaN,
    [double]$MakepadProjectionAreaScaleUv = [double]::NaN,
    [double]$MakepadProjectionAreaScaleX = [double]::NaN,
    [double]$MakepadProjectionAreaScaleY = [double]::NaN,
    [double]$VulkanProjectionAreaScaleUv = [double]::NaN,
    [double]$VulkanDirectProjectionAreaScaleUv = [double]::NaN,
    [double]$VulkanBrokerProjectionAreaScaleUv = [double]::NaN,
    [double]$GlesDirectProjectionAreaScaleUv = [double]::NaN,
    [double]$GlesBrokerProjectionAreaScaleUv = [double]::NaN,
    [double]$MakepadDirectProjectionAreaScaleUv = [double]::NaN,
    [double]$MakepadBrokerProjectionAreaScaleUv = [double]::NaN,
    [double]$MakepadDirectProjectionAreaScaleX = [double]::NaN,
    [double]$MakepadDirectProjectionAreaScaleY = [double]::NaN,
    [double]$MakepadBrokerProjectionAreaScaleX = [double]::NaN,
    [double]$MakepadBrokerProjectionAreaScaleY = [double]::NaN,
    [double]$VulkanCameraProjectionScale = 1.0,
    [double]$VulkanDirectCameraProjectionScale = [double]::NaN,
    [double]$VulkanBrokerCameraProjectionScale = [double]::NaN,
    [double]$VulkanCameraPreviewFovYDegrees = [double]::NaN,
    [double]$VulkanDirectCameraPreviewFovYDegrees = [double]::NaN,
    [double]$VulkanBrokerCameraPreviewFovYDegrees = [double]::NaN,
    [double]$VulkanCameraRawOverlayOverscan = [double]::NaN,
    [double]$VulkanDirectCameraRawOverlayOverscan = [double]::NaN,
    [double]$VulkanBrokerCameraRawOverlayOverscan = [double]::NaN,
    [double]$VulkanCameraFullViewOverlayOverscan = [double]::NaN,
    [double]$VulkanDirectCameraFullViewOverlayOverscan = [double]::NaN,
    [double]$VulkanBrokerCameraFullViewOverlayOverscan = [double]::NaN,
    [double]$VulkanXrRenderScale = 1.0,
    [double]$VulkanDirectXrRenderScale = [double]::NaN,
    [double]$VulkanBrokerXrRenderScale = [double]::NaN,
    [double]$MakepadProjectionScale = 1.0,
    [double]$MakepadDirectProjectionScale = [double]::NaN,
    [double]$MakepadBrokerProjectionScale = [double]::NaN,
    [double]$MakepadXrRenderScale = 1.0,
    [double]$MakepadDirectXrRenderScale = [double]::NaN,
    [double]$MakepadBrokerXrRenderScale = [double]::NaN,
    [double]$VulkanProjectionAreaRadiusXUv = 0.5,
    [double]$VulkanProjectionAreaRadiusYUv = 0.5,
    [double]$VulkanProjectionAreaCornerRadiusUv = 0.0,
    [double]$GlesProjectionAreaRadiusXUv = 0.5,
    [double]$GlesProjectionAreaRadiusYUv = 0.5,
    [double]$GlesProjectionAreaCornerRadiusUv = 0.0,
    [double]$MakepadProjectionAreaRadiusXUv = 0.5,
    [double]$MakepadProjectionAreaRadiusYUv = 0.5,
    [double]$MakepadProjectionAreaCornerRadiusUv = 0.0,
    [double]$ProjectionAreaOpacity = 1.0,
    [double]$ProjectionBorderOpacity = 1.0,
    [string]$GlesCameraColorMatrix = "1;0;0;0;1;0;0;0;1",
    [string]$GlesCameraColorOffset = "0;0;0",
    [double]$GlesCameraColorContrast = 1.0,
    [double]$GlesCameraColorBrightness = 0.0,
    [double]$GlesCameraColorSaturation = 1.0,
    [switch]$EnableNativePassthroughUnderlay,
    [switch]$EnableStayAwakeGuard,
    [switch]$RestoreStayAwakeGuard,
    [switch]$SkipLaneAppForceStop,
    [int]$LaneAppForceStopSettleSeconds = 2,
    [switch]$CaptureHzdbScreencap,
    [switch]$ContinueOnError
)

$ErrorActionPreference = "Stop"

$allModes = @(
    "vulkan-hwb-direct-camera2-raw",
    "vulkan-hwb-broker-h264-raw",
    "gles-oes-direct-camera2-raw",
    "gles-oes-broker-h264-raw",
    "makepad-cpuyuv-direct-camera2-raw",
    "makepad-cpuyuv-broker-h264-raw"
)

if ($Mode.Count -eq 0) {
    $Mode = $allModes
}
elseif ($Mode.Count -eq 1 -and $Mode[0].Contains(",")) {
    $Mode = @($Mode[0].Split(",") | ForEach-Object { $_.Trim() } | Where-Object { $_ })
}

$unknownModes = @($Mode | Where-Object { $allModes -notcontains $_ })
if ($unknownModes.Count -gt 0) {
    throw "Unknown mode(s): $($unknownModes -join ', '). Valid modes: $($allModes -join ', ')"
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$cameraProfileRunner = Join-Path $PSScriptRoot "Invoke-QuestCameraProfileRun.ps1"
$makepadRunner = Join-Path $repoRoot "examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1"
$compositeCatalog = Join-Path $repoRoot "examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json"
$glesCatalog = Join-Path $repoRoot "examples\quest-gl-openxr-video-stack-apk\catalog\rusty-xr-quest-gl-openxr-video-stack.catalog.json"
$resolvedRunRoot = if ([System.IO.Path]::IsPathRooted($RunRoot)) {
    $RunRoot
}
else {
    Join-Path $repoRoot $RunRoot
}
$sessionId = Get-Date -Format "yyyyMMdd-HHmmss"
$sessionRoot = Join-Path $resolvedRunRoot $sessionId
New-Item -ItemType Directory -Force -Path $sessionRoot | Out-Null

$installed = @{
    composite = $false
    gles = $false
    makepad = $false
}
$results = [System.Collections.Generic.List[object]]::new()

function Invoke-AdbText {
    param([string[]]$Arguments)
    $adbArgs = @()
    if ($Serial) {
        $adbArgs += @("-s", $Serial)
    }
    $adbArgs += $Arguments
    & $Adb @adbArgs
}

function Save-TextCommand {
    param(
        [string]$Path,
        [scriptblock]$Command
    )
    try {
        & $Command 2>&1 | Out-File -FilePath $Path -Encoding UTF8
    }
    catch {
        $_.Exception.Message | Out-File -FilePath $Path -Encoding UTF8
    }
}

function Save-StateSnapshot {
    param([string]$Label)
    $safeLabel = $Label -replace '[^A-Za-z0-9_.-]', '_'
    $snapshotRoot = Join-Path $sessionRoot "state-snapshots\$safeLabel"
    New-Item -ItemType Directory -Force -Path $snapshotRoot | Out-Null
    Save-TextCommand -Path (Join-Path $snapshotRoot "adb-get-state.txt") -Command {
        Invoke-AdbText -Arguments @("get-state")
    }
    Save-TextCommand -Path (Join-Path $snapshotRoot "dumpsys-power.txt") -Command {
        Invoke-AdbText -Arguments @("shell", "dumpsys", "power")
    }
    Save-TextCommand -Path (Join-Path $snapshotRoot "dumpsys-vrpowermanager.txt") -Command {
        Invoke-AdbText -Arguments @("shell", "dumpsys", "vrpowermanager")
    }
    Save-TextCommand -Path (Join-Path $snapshotRoot "stay-on-while-plugged-in.txt") -Command {
        Invoke-AdbText -Arguments @("shell", "settings", "get", "global", "stay_on_while_plugged_in")
    }
    Save-TextCommand -Path (Join-Path $snapshotRoot "activity-activities.txt") -Command {
        Invoke-AdbText -Arguments @("shell", "dumpsys", "activity", "activities")
    }
    Save-TextCommand -Path (Join-Path $snapshotRoot "window-windows.txt") -Command {
        Invoke-AdbText -Arguments @("shell", "dumpsys", "window", "windows")
    }
    Save-TextCommand -Path (Join-Path $snapshotRoot "broker-status.json") -Command {
        (Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:8765/status" -TimeoutSec 3).Content
    }
    Save-TextCommand -Path (Join-Path $snapshotRoot "broker-clock-now.json") -Command {
        (Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:8765/clock/now" -TimeoutSec 3).Content
    }
    Save-TextCommand -Path (Join-Path $snapshotRoot "broker-clock-health.json") -Command {
        (Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:8765/clock/health" -TimeoutSec 3).Content
    }
}

function Get-FirstRegexValue {
    param(
        [string]$Text,
        [string]$Pattern
    )
    if ($Text -match $Pattern) {
        return $Matches[1]
    }
    return ""
}

function Get-StateSnapshotSummary {
    param([string]$Label)
    $safeLabel = $Label -replace '[^A-Za-z0-9_.-]', '_'
    $snapshotRoot = Join-Path $sessionRoot "state-snapshots\$safeLabel"
    $powerPath = Join-Path $snapshotRoot "dumpsys-power.txt"
    $vrPath = Join-Path $snapshotRoot "dumpsys-vrpowermanager.txt"
    $stayPath = Join-Path $snapshotRoot "stay-on-while-plugged-in.txt"
    $powerText = if (Test-Path -LiteralPath $powerPath) { Get-Content -Raw -Path $powerPath } else { "" }
    $vrText = if (Test-Path -LiteralPath $vrPath) { Get-Content -Raw -Path $vrPath } else { "" }
    $staySetting = if (Test-Path -LiteralPath $stayPath) { ((Get-Content -Raw -Path $stayPath) -split "`r?`n" | Select-Object -First 1).Trim() } else { "" }

    return [pscustomobject]@{
        label = $Label
        artifactRoot = $snapshotRoot
        wakefulness = Get-FirstRegexValue -Text $powerText -Pattern 'mWakefulness=([^\r\n]+)'
        stayOn = Get-FirstRegexValue -Text $powerText -Pattern 'mStayOn=([^\r\n]+)'
        stayOnWhilePluggedIn = $staySetting
        proximityPositive = Get-FirstRegexValue -Text $powerText -Pattern 'mProximityPositive=([^\r\n]+)'
        lastSleepReason = Get-FirstRegexValue -Text $powerText -Pattern 'mLastSleepReason=([^\r\n]+)'
        vrState = Get-FirstRegexValue -Text $vrText -Pattern '(?m)^State:\s*([^\r\n]+)'
        virtualProximityState = Get-FirstRegexValue -Text $vrText -Pattern 'Virtual proximity state:\s*([^\r\n]+)'
        mountWakeLockCount = Get-FirstRegexValue -Text $vrText -Pattern 'MountWakeLock count:\s*([^\r\n]+)'
        hasGoToSleep = [bool]($vrText -match 'Calling goToSleep\(\)')
        hasMountWakeLockFalseIdle = [bool]($vrText -match 'onDeviceIdle: state: HEADSET_MOUNTED, forceUnmountWakelock: false, mountWakelock: false')
    }
}

function Get-StateIssueSummary {
    param(
        [object]$Before,
        [object]$After
    )
    $issues = [System.Collections.Generic.List[string]]::new()
    if ($After.wakefulness -eq "Asleep") {
        $issues.Add("after wakefulness is Asleep")
    }
    if ($After.vrState -eq "STANDBY") {
        $issues.Add("after VR power state is STANDBY")
    }
    if ($Before.wakefulness -and $After.wakefulness -and $Before.wakefulness -ne $After.wakefulness) {
        $issues.Add(("wakefulness changed {0}->{1}" -f $Before.wakefulness, $After.wakefulness))
    }
    if ($Before.vrState -and $After.vrState -and $Before.vrState -ne $After.vrState) {
        $issues.Add(("VR state changed {0}->{1}" -f $Before.vrState, $After.vrState))
    }
    if ($After.hasMountWakeLockFalseIdle) {
        $issues.Add("VR power log contains mountWakelock=false idle")
    }
    if ($After.hasGoToSleep) {
        $issues.Add("VR power log contains goToSleep")
    }
    return ($issues -join "; ")
}

function Restart-BrokerBeforeMode {
    param([string]$ModeId)
    if (-not $RestartBrokerBeforeBrokerModes -or -not $ModeId.Contains("broker-h264")) {
        return
    }

    $restartRoot = Join-Path $sessionRoot ("broker-restarts\" + ($ModeId -replace '[^A-Za-z0-9_.-]', '_'))
    New-Item -ItemType Directory -Force -Path $restartRoot | Out-Null
    Save-TextCommand -Path (Join-Path $restartRoot "before-broker-status.json") -Command {
        (Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:8765/status" -TimeoutSec 3).Content
    }

    Invoke-AdbText -Arguments @("shell", "am", "force-stop", $BrokerPackageName) |
        Out-File -FilePath (Join-Path $restartRoot "force-stop.txt") -Encoding UTF8
    Start-Sleep -Seconds 2

    $activity = if ($BrokerActivityName.StartsWith(".")) {
        "$BrokerPackageName/$BrokerActivityName"
    }
    elseif ($BrokerActivityName.Contains("/")) {
        $BrokerActivityName
    }
    else {
        "$BrokerPackageName/$BrokerActivityName"
    }
    Invoke-AdbText -Arguments @("shell", "am", "start", "-n", $activity) |
        Out-File -FilePath (Join-Path $restartRoot "start.txt") -Encoding UTF8
    Start-Sleep -Seconds ([Math]::Max(1, $BrokerRestartSettleSeconds))

    Save-TextCommand -Path (Join-Path $restartRoot "after-broker-status.json") -Command {
        (Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:8765/status" -TimeoutSec 5).Content
    }
}

function Stop-LaneAppsBeforeMode {
    param([string]$ModeId)
    if ($SkipLaneAppForceStop) {
        return
    }

    $stopRoot = Join-Path $sessionRoot ("lane-app-force-stops\" + ($ModeId -replace '[^A-Za-z0-9_.-]', '_'))
    New-Item -ItemType Directory -Force -Path $stopRoot | Out-Null

    $packages = [System.Collections.Generic.List[string]]::new()
    $packages.Add("com.example.rustyxr.composite")
    $packages.Add("com.example.rustyxr.opengles")
    if ($MakepadPackageName) {
        $packages.Add($MakepadPackageName)
    }

    $seen = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($packageName in $packages) {
        if ([string]::IsNullOrWhiteSpace($packageName) -or -not $seen.Add($packageName)) {
            continue
        }
        $safeName = $packageName -replace '[^A-Za-z0-9_.-]', '_'
        Invoke-AdbText -Arguments @("shell", "am", "force-stop", $packageName) |
            Out-File -FilePath (Join-Path $stopRoot "$safeName.txt") -Encoding UTF8
    }

    if ($LaneAppForceStopSettleSeconds -gt 0) {
        Start-Sleep -Seconds $LaneAppForceStopSettleSeconds
    }
}

function Set-StayAwakeGuard {
    $guardRoot = Join-Path $sessionRoot "awake-guard"
    New-Item -ItemType Directory -Force -Path $guardRoot | Out-Null
    $before = ((Invoke-AdbText -Arguments @("shell", "settings", "get", "global", "stay_on_while_plugged_in")) -join "").Trim()
    $before | Set-Content -Path (Join-Path $guardRoot "before-stay-on-while-plugged-in.txt") -Encoding UTF8
    Save-TextCommand -Path (Join-Path $guardRoot "before-dumpsys-power.txt") -Command {
        Invoke-AdbText -Arguments @("shell", "dumpsys", "power")
    }
    Invoke-AdbText -Arguments @("shell", "svc", "power", "stayon", "true") |
        Out-File -FilePath (Join-Path $guardRoot "svc-power-stayon-true.txt") -Encoding UTF8
    $after = ((Invoke-AdbText -Arguments @("shell", "settings", "get", "global", "stay_on_while_plugged_in")) -join "").Trim()
    $after | Set-Content -Path (Join-Path $guardRoot "after-stay-on-while-plugged-in.txt") -Encoding UTF8
    Save-TextCommand -Path (Join-Path $guardRoot "after-dumpsys-power.txt") -Command {
        Invoke-AdbText -Arguments @("shell", "dumpsys", "power")
    }
    return [pscustomobject]@{
        beforeStayOnWhilePluggedIn = $before
        afterStayOnWhilePluggedIn = $after
        artifactRoot = $guardRoot
    }
}

function Restore-StayAwakeGuard {
    param([object]$GuardState)
    if (-not $GuardState) {
        return
    }
    $guardRoot = $GuardState.artifactRoot
    $previous = [string]$GuardState.beforeStayOnWhilePluggedIn
    if (-not $previous) {
        return
    }
    Invoke-AdbText -Arguments @("shell", "settings", "put", "global", "stay_on_while_plugged_in", $previous) |
        Out-File -FilePath (Join-Path $guardRoot "restore-stay-on-while-plugged-in.txt") -Encoding UTF8
    $restored = ((Invoke-AdbText -Arguments @("shell", "settings", "get", "global", "stay_on_while_plugged_in")) -join "").Trim()
    $restored | Set-Content -Path (Join-Path $guardRoot "restored-stay-on-while-plugged-in.txt") -Encoding UTF8
}

function Resolve-OptionalPath {
    param([string]$Path)
    if (-not $Path) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))
}

$stayAwakeGuardState = $null
if ($EnableStayAwakeGuard) {
    $stayAwakeGuardState = Set-StayAwakeGuard
}

function Join-OverrideValues {
    param([string[]]$Values)
    return (@($Values | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ",")
}

function Format-InvariantDouble {
    param([double]$Value)
    return $Value.ToString("0.######", [Globalization.CultureInfo]::InvariantCulture)
}

function Format-OptionalInvariantDouble {
    param([double]$Value)
    if ([double]::IsNaN($Value)) {
        return "app default"
    }
    return (Format-InvariantDouble -Value $Value)
}

function Resolve-ProjectionAreaOffsetYUv {
    param([double]$RendererValue)
    if ([double]::IsNaN($RendererValue)) {
        return $ProjectionAreaOffsetYUv
    }
    return $RendererValue
}

function Resolve-ModeProjectionAreaOffsetYUv {
    param(
        [double]$RendererValue,
        [double]$ModeValue
    )
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    return (Resolve-ProjectionAreaOffsetYUv -RendererValue $RendererValue)
}

function Resolve-BrokerModeProjectionAreaOffsetYUv {
    param(
        [double]$RendererValue,
        [double]$ModeValue,
        [double]$FullFrameRendererValue
    )
    if ($BrokerH264SyntheticProjectionProfile -eq "full-frame-diagnostic") {
        if (-not [double]::IsNaN($FullFrameRendererValue)) {
            return $FullFrameRendererValue
        }
        if (-not [double]::IsNaN($FullFrameBrokerProjectionAreaOffsetYUv)) {
            return $FullFrameBrokerProjectionAreaOffsetYUv
        }
    }
    return (Resolve-ModeProjectionAreaOffsetYUv -RendererValue $RendererValue -ModeValue $ModeValue)
}

function Resolve-ProjectionAreaScaleUv {
    param([double]$RendererValue)
    if ([double]::IsNaN($RendererValue)) {
        return $ProjectionAreaScaleUv
    }
    return $RendererValue
}

function Resolve-ModeProjectionAreaScaleUv {
    param(
        [double]$RendererValue,
        [double]$ModeValue
    )
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    return (Resolve-ProjectionAreaScaleUv -RendererValue $RendererValue)
}

function Resolve-MakepadProjectionAreaScaleAxis {
    param(
        [double]$RendererValue,
        [double]$ModeValue,
        [double]$FallbackScaleUv
    )
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    if (-not [double]::IsNaN($RendererValue)) {
        return $RendererValue
    }
    return $FallbackScaleUv
}

function Resolve-VulkanCameraProjectionScale {
    param([double]$ModeValue)
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    return $VulkanCameraProjectionScale
}

function Resolve-VulkanXrRenderScale {
    param([double]$ModeValue)
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    return $VulkanXrRenderScale
}

function Resolve-MakepadProjectionScale {
    param([double]$ModeValue)
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    return $MakepadProjectionScale
}

function Resolve-MakepadXrRenderScale {
    param([double]$ModeValue)
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    return $MakepadXrRenderScale
}

function Resolve-VulkanProjectionAreaScaleUv {
    param([double]$ModeValue)
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    return (Resolve-ProjectionAreaScaleUv -RendererValue $VulkanProjectionAreaScaleUv)
}

function Resolve-VulkanOptionalDouble {
    param(
        [double]$RendererValue,
        [double]$ModeValue
    )
    if (-not [double]::IsNaN($ModeValue)) {
        return $ModeValue
    }
    if (-not [double]::IsNaN($RendererValue)) {
        return $RendererValue
    }
    return [double]::NaN
}

function Get-VulkanProjectionBorderOverride {
    param(
        [double]$OffsetYUv = (Resolve-ProjectionAreaOffsetYUv -RendererValue $VulkanProjectionAreaOffsetYUv),
        [double]$CameraProjectionScale = $VulkanCameraProjectionScale,
        [double]$XrRenderScale = $VulkanXrRenderScale,
        [double]$ProjectionAreaScaleUv = (Resolve-ProjectionAreaScaleUv -RendererValue $VulkanProjectionAreaScaleUv),
        [double]$CameraPreviewFovYDegrees = [double]::NaN,
        [double]$CameraRawOverlayOverscan = [double]::NaN,
        [double]$CameraFullViewOverlayOverscan = [double]::NaN
    )
    $blurRadius = Format-InvariantDouble -Value $BlurRadiusPx
    $offsetY = Format-InvariantDouble -Value $OffsetYUv
    $projectionScale = Format-InvariantDouble -Value $CameraProjectionScale
    $xrRenderScale = Format-InvariantDouble -Value $XrRenderScale
    $projectionAreaScaleUv = Format-InvariantDouble -Value $ProjectionAreaScaleUv
    $areaRadiusX = Format-InvariantDouble -Value $VulkanProjectionAreaRadiusXUv
    $areaRadiusY = Format-InvariantDouble -Value $VulkanProjectionAreaRadiusYUv
    $areaCornerRadius = Format-InvariantDouble -Value $VulkanProjectionAreaCornerRadiusUv
    $areaOpacity = Format-InvariantDouble -Value $ProjectionAreaOpacity
    $borderOpacity = Format-InvariantDouble -Value $ProjectionBorderOpacity
    $commonValues = [System.Collections.Generic.List[string]]::new()
    $commonValues.Add("rustyxr.xrRenderScale=$xrRenderScale")
    $commonValues.Add("rustyxr.cameraProjectionScale=$projectionScale")
    $commonValues.Add("rustyxr.cameraProjectionAreaScaleUv=$projectionAreaScaleUv")
    $commonValues.Add("rustyxr.cameraProjectionAreaOffsetYUv=$offsetY")
    $commonValues.Add("rustyxr.cameraProjectionAreaRadiusXUv=$areaRadiusX")
    $commonValues.Add("rustyxr.cameraProjectionAreaRadiusYUv=$areaRadiusY")
    $commonValues.Add("rustyxr.cameraProjectionAreaCornerRadiusUv=$areaCornerRadius")
    $commonValues.Add("rustyxr.cameraProjectionAreaOpacity=$areaOpacity")
    $commonValues.Add("rustyxr.cameraProjectionBorderOpacity=$borderOpacity")
    if (-not [double]::IsNaN($CameraPreviewFovYDegrees)) {
        $commonValues.Add(("rustyxr.cameraPreviewFovYDegrees={0}" -f (Format-InvariantDouble -Value $CameraPreviewFovYDegrees)))
    }
    if (-not [double]::IsNaN($CameraRawOverlayOverscan)) {
        $commonValues.Add(("rustyxr.cameraRawOverlayOverscan={0}" -f (Format-InvariantDouble -Value $CameraRawOverlayOverscan)))
    }
    if (-not [double]::IsNaN($CameraFullViewOverlayOverscan)) {
        $commonValues.Add(("rustyxr.cameraFullViewOverlayOverscan={0}" -f (Format-InvariantDouble -Value $CameraFullViewOverlayOverscan)))
    }
    $commonOverride = $commonValues.ToArray() -join ","
    $passthroughOverride = if ($EnableNativePassthroughUnderlay -or $ProjectionBorderPolicy -eq "passthrough-underlay" -or $ProjectionAreaOpacity -lt 1.0 -or $ProjectionBorderOpacity -lt 1.0) {
        "rustyxr.openxrPassthroughProbe=underlay"
    }
    else {
        "rustyxr.openxrPassthroughProbe=off"
    }
    if ($ProcessingLayer -eq "blur") {
        if ($ProjectionBorderPolicy -eq "passthrough-underlay") {
            return "rustyxr.cameraPipelinePreset=raw-projection-blur-underlay-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-blur-underlay,$passthroughOverride,rustyxr.cameraBlurRadiusPx=$blurRadius,$commonOverride"
        }
        return "rustyxr.cameraPipelinePreset=raw-projection-blur-solid-red-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-blur-solid-red,$passthroughOverride,rustyxr.cameraBlurRadiusPx=$blurRadius,$commonOverride"
    }
    if ($ProjectionBorderPolicy -eq "passthrough-underlay") {
        return "rustyxr.cameraPipelinePreset=raw-projection-underlay-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-underlay,$passthroughOverride,$commonOverride"
    }
    return "rustyxr.cameraPipelinePreset=raw-projection-solid-red-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-solid-red,$passthroughOverride,$commonOverride"
}

function Get-GlesProjectionBorderOverride {
    param(
        [double]$OffsetYUv = (Resolve-ProjectionAreaOffsetYUv -RendererValue $GlesProjectionAreaOffsetYUv),
        [double]$ScaleUv = (Resolve-ProjectionAreaScaleUv -RendererValue $GlesProjectionAreaScaleUv)
    )
    $blurRadius = Format-InvariantDouble -Value $BlurRadiusPx
    $offsetY = Format-InvariantDouble -Value $OffsetYUv
    $scaleUv = Format-InvariantDouble -Value $ScaleUv
    $areaRadiusX = Format-InvariantDouble -Value $GlesProjectionAreaRadiusXUv
    $areaRadiusY = Format-InvariantDouble -Value $GlesProjectionAreaRadiusYUv
    $cornerRadius = Format-InvariantDouble -Value $GlesProjectionAreaCornerRadiusUv
    $areaOpacity = Format-InvariantDouble -Value $ProjectionAreaOpacity
    $borderOpacity = Format-InvariantDouble -Value $ProjectionBorderOpacity
    $colorContrast = Format-InvariantDouble -Value $GlesCameraColorContrast
    $colorBrightness = Format-InvariantDouble -Value $GlesCameraColorBrightness
    $colorSaturation = Format-InvariantDouble -Value $GlesCameraColorSaturation
    return "rustyxr.projectionBorderPolicy=$ProjectionBorderPolicy,rustyxr.processingLayer=$ProcessingLayer,rustyxr.cameraBlurRadiusPx=$blurRadius,rustyxr.projectionAreaOffsetYUv=$offsetY,rustyxr.projectionAreaScaleUv=$scaleUv,rustyxr.projectionAreaRadiusXUv=$areaRadiusX,rustyxr.projectionAreaRadiusYUv=$areaRadiusY,rustyxr.projectionAreaCornerRadiusUv=$cornerRadius,rustyxr.projectionAreaOpacity=$areaOpacity,rustyxr.projectionBorderOpacity=$borderOpacity,rustyxr.cameraColorMatrix=$GlesCameraColorMatrix,rustyxr.cameraColorOffset=$GlesCameraColorOffset,rustyxr.cameraColorContrast=$colorContrast,rustyxr.cameraColorBrightness=$colorBrightness,rustyxr.cameraColorSaturation=$colorSaturation"
}

function Get-BrokerH264Override {
    $values = [System.Collections.Generic.List[string]]::new()
    $values.Add("rustyxr.brokerH264SourceMode=$BrokerH264SourceMode")
    $values.Add("rustyxr.brokerH264SyntheticPattern=$BrokerH264SyntheticPattern")
    $values.Add("rustyxr.brokerH264SyntheticProjectionProfile=$BrokerH264SyntheticProjectionProfile")
    $values.Add("rustyxr.brokerH264StreamPort=$BrokerH264LeftStreamPort")
    $values.Add("rustyxr.brokerH264RightStreamPort=$BrokerH264RightStreamPort")
    $values.Add("rustyxr.brokerH264CaptureMs=$BrokerH264CaptureMs")
    $values.Add("rustyxr.brokerH264MaxPackets=$BrokerH264MaxPackets")
    $values.Add("rustyxr.brokerH264FrameRateHz=$BrokerH264FrameRateHz")
    $values.Add("rustyxr.brokerH264Width=$BrokerH264Width")
    $values.Add("rustyxr.brokerH264Height=$BrokerH264Height")
    $values.Add("rustyxr.brokerH264BitrateBps=$BrokerH264BitrateBps")
    $values.Add("rustyxr.brokerH264LiveStream=true")
    $values.Add("rustyxr.brokerH264LiveDecode=true")
    if ($BrokerH264SourceMode -eq "broker-camera" -or
        ($BrokerH264SourceMode -eq "broker-synthetic" -and $BrokerH264SyntheticProjectionProfile -eq "camera-matched")) {
        $values.Add("rustyxr.brokerH264LeftCameraId=$BrokerH264LeftCameraId")
        $values.Add("rustyxr.brokerH264RightCameraId=$BrokerH264RightCameraId")
    }
    return ($values.ToArray() -join ",")
}

function Get-CommonQuestProfileArgs {
    param([string]$ModeRunRoot)
    $common = [System.Collections.Generic.List[string]]::new()
    if ($Serial) {
        $common.Add("-Serial")
        $common.Add($Serial)
    }
    $common.Add("-Adb")
    $common.Add($Adb)
    $common.Add("-RunRoot")
    $common.Add($ModeRunRoot)
    $common.Add("-WarmupSeconds")
    $common.Add([string]$WarmupSeconds)
    $common.Add("-FreshnessFrames")
    $common.Add([string]$FreshnessFrames)
    $common.Add("-FreshnessIntervalMs")
    $common.Add([string]$FreshnessIntervalMs)
    if ($CaptureHzdbScreencap) {
        $common.Add("-CaptureHzdbScreencap")
    }
    return $common.ToArray()
}

function Invoke-QuestProfileMode {
    param(
        [string]$ModeId,
        [string]$Architecture,
        [string]$Catalog,
        [string]$AppId,
        [string]$DeviceProfile,
        [string]$RuntimeProfile,
        [string]$Apk,
        [string]$InstallKey,
        [string]$Override = ""
    )

    $modeRoot = Join-Path $sessionRoot $ModeId
    New-Item -ItemType Directory -Force -Path $modeRoot | Out-Null
    Save-StateSnapshot -Label "before-$ModeId"
    $stateBefore = Get-StateSnapshotSummary -Label "before-$ModeId"

    $argList = [System.Collections.Generic.List[string]]::new()
    foreach ($item in (Get-CommonQuestProfileArgs -ModeRunRoot $modeRoot)) {
        $argList.Add($item)
    }
    $argList.Add("-Catalog")
    $argList.Add($Catalog)
    $argList.Add("-AppId")
    $argList.Add($AppId)
    $argList.Add("-DeviceProfile")
    $argList.Add($DeviceProfile)
    $argList.Add("-RuntimeProfile")
    $argList.Add($RuntimeProfile)
    if ($Override) {
        $argList.Add("-Override")
        $argList.Add($Override)
    }
    if ($Install -and -not $installed[$InstallKey]) {
        $resolvedApk = Resolve-OptionalPath -Path $Apk
        if (-not $resolvedApk -or -not (Test-Path -LiteralPath $resolvedApk)) {
            throw "Install requested for $ModeId, but APK was not found: $Apk"
        }
        $argList.Add("-Install")
        $argList.Add("-Apk")
        $argList.Add($resolvedApk)
        $installed[$InstallKey] = $true
    }

    $commandLine = @("powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $cameraProfileRunner) + $argList
    $commandLine | Set-Content -Path (Join-Path $modeRoot "command.txt") -Encoding UTF8

    $status = "completed"
    $errorMessage = ""
    try {
        & powershell -NoProfile -ExecutionPolicy Bypass -File $cameraProfileRunner @argList
        if ($LASTEXITCODE -ne 0) {
            throw "Quest camera profile runner failed with exit code $LASTEXITCODE."
        }
    }
    catch {
        $status = "failed"
        $errorMessage = $_.Exception.Message
        if (-not $ContinueOnError) {
            throw
        }
    }
    Save-StateSnapshot -Label "after-$ModeId"
    $stateAfter = Get-StateSnapshotSummary -Label "after-$ModeId"
    $stateIssues = Get-StateIssueSummary -Before $stateBefore -After $stateAfter

    $latestRun = Get-ChildItem -Path $modeRoot -Directory -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    $results.Add([pscustomobject]@{
            mode = $ModeId
            architecture = $Architecture
            status = $status
            error = $errorMessage
            runtimeProfile = $RuntimeProfile
            artifactRoot = $modeRoot
            latestRun = if ($latestRun) { $latestRun.FullName } else { "" }
            stateBefore = $stateBefore
            stateAfter = $stateAfter
            stateIssues = $stateIssues
        })
}

function Invoke-MakepadMode {
    param(
        [string]$ModeId,
        [string]$Architecture,
        [string]$BrokerSourceMode = "",
        [double]$OffsetYUv = (Resolve-ProjectionAreaOffsetYUv -RendererValue $MakepadProjectionAreaOffsetYUv),
        [double]$ScaleUv = (Resolve-ProjectionAreaScaleUv -RendererValue $MakepadProjectionAreaScaleUv),
        [double]$ScaleX = [double]::NaN,
        [double]$ScaleY = [double]::NaN,
        [double]$ProjectionScale = $MakepadProjectionScale,
        [double]$XrRenderScale = $MakepadXrRenderScale
    )

    if (-not $MakepadApk -or -not $MakepadPackageName -or -not $MakepadLauncherActivity -or -not $MakepadXrActivity) {
        throw "Makepad modes require -MakepadApk, -MakepadPackageName, -MakepadLauncherActivity, and -MakepadXrActivity."
    }

    $modeRoot = Join-Path $sessionRoot $ModeId
    New-Item -ItemType Directory -Force -Path $modeRoot | Out-Null
    Save-StateSnapshot -Label "before-$ModeId"
    $stateBefore = Get-StateSnapshotSummary -Label "before-$ModeId"
    $resolvedScaleX = if ([double]::IsNaN($ScaleX)) { $ScaleUv } else { $ScaleX }
    $resolvedScaleY = if ([double]::IsNaN($ScaleY)) { $ScaleUv } else { $ScaleY }

    $argList = [System.Collections.Generic.List[string]]::new()
    $argList.Add("-Serial")
    $argList.Add($Serial)
    $argList.Add("-Apk")
    $argList.Add((Resolve-OptionalPath -Path $MakepadApk))
    $argList.Add("-PackageName")
    $argList.Add($MakepadPackageName)
    $argList.Add("-LauncherActivity")
    $argList.Add($MakepadLauncherActivity)
    $argList.Add("-XrActivity")
    $argList.Add($MakepadXrActivity)
    $argList.Add("-OutDir")
    $argList.Add($modeRoot)
    $argList.Add("-SampleSeconds")
    $argList.Add([string]$SampleSeconds)
    $argList.Add("-FreshnessFrames")
    $argList.Add([string]$FreshnessFrames)
    $argList.Add("-FreshnessIntervalSeconds")
    $argList.Add([string][Math]::Max(1, [int][Math]::Round($FreshnessIntervalMs / 1000.0)))
    $argList.Add("-ProjectionBorderPolicy")
    $argList.Add($ProjectionBorderPolicy)
    $argList.Add("-ProcessingLayer")
    $argList.Add($ProcessingLayer)
    $argList.Add("-BlurRadiusPx")
    $argList.Add((Format-InvariantDouble -Value $BlurRadiusPx))
    $argList.Add("-ProjectionScale")
    $argList.Add((Format-InvariantDouble -Value $ProjectionScale))
    $argList.Add("-XrRenderScale")
    $argList.Add((Format-InvariantDouble -Value $XrRenderScale))
    $argList.Add("-ProjectionAreaOffsetYUv")
    $argList.Add((Format-InvariantDouble -Value $OffsetYUv))
    $argList.Add("-ProjectionAreaScaleX")
    $argList.Add((Format-InvariantDouble -Value $resolvedScaleX))
    $argList.Add("-ProjectionAreaScaleY")
    $argList.Add((Format-InvariantDouble -Value $resolvedScaleY))
    $argList.Add("-ProjectionAreaRadiusXUv")
    $argList.Add((Format-InvariantDouble -Value $MakepadProjectionAreaRadiusXUv))
    $argList.Add("-ProjectionAreaRadiusYUv")
    $argList.Add((Format-InvariantDouble -Value $MakepadProjectionAreaRadiusYUv))
    $argList.Add("-ProjectionAreaCornerRadiusUv")
    $argList.Add((Format-InvariantDouble -Value $MakepadProjectionAreaCornerRadiusUv))
    $argList.Add("-ProjectionAreaOpacity")
    $argList.Add((Format-InvariantDouble -Value $ProjectionAreaOpacity))
    $argList.Add("-ProjectionBorderOpacity")
    $argList.Add((Format-InvariantDouble -Value $ProjectionBorderOpacity))
    if ($EnableNativePassthroughUnderlay) {
        $argList.Add("-EnableNativePassthrough")
    }

    if ($BrokerSourceMode -eq "broker-camera") {
        $argList.Add("-UseBrokerH264Camera")
        $argList.Add("-BrokerH264LeftCameraId")
        $argList.Add($BrokerH264LeftCameraId)
        $argList.Add("-BrokerH264RightCameraId")
        $argList.Add($BrokerH264RightCameraId)
    }
    elseif ($BrokerSourceMode -eq "broker-synthetic") {
        $argList.Add("-UseBrokerH264Synthetic")
        $argList.Add("-BrokerH264SyntheticPattern")
        $argList.Add($BrokerH264SyntheticPattern)
        $argList.Add("-BrokerH264SyntheticProjectionProfile")
        $argList.Add($BrokerH264SyntheticProjectionProfile)
    }

    if ($BrokerSourceMode) {
        $argList.Add("-BrokerH264CaptureMs")
        $argList.Add([string]$BrokerH264CaptureMs)
        $argList.Add("-BrokerH264MaxPackets")
        $argList.Add([string]$BrokerH264MaxPackets)
        $argList.Add("-BrokerH264FrameRateHz")
        $argList.Add([string]$BrokerH264FrameRateHz)
        $argList.Add("-BrokerH264Width")
        $argList.Add([string]$BrokerH264Width)
        $argList.Add("-BrokerH264Height")
        $argList.Add([string]$BrokerH264Height)
        $argList.Add("-BrokerH264BitrateBps")
        $argList.Add([string]$BrokerH264BitrateBps)
        $argList.Add("-BrokerH264LeftStreamPort")
        $argList.Add([string]$BrokerH264LeftStreamPort)
        $argList.Add("-BrokerH264RightStreamPort")
        $argList.Add([string]$BrokerH264RightStreamPort)
    }

    if (-not $Install -or $installed["makepad"]) {
        $argList.Add("-SkipInstall")
    }
    else {
        $installed["makepad"] = $true
    }
    $argList.Add("-PreferDirectVrActivity")

    $commandLine = @("powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $makepadRunner) + $argList
    $commandLine | Set-Content -Path (Join-Path $modeRoot "command.txt") -Encoding UTF8

    $status = "completed"
    $errorMessage = ""
    try {
        & powershell -NoProfile -ExecutionPolicy Bypass -File $makepadRunner @argList
        if ($LASTEXITCODE -ne 0) {
            throw "Makepad device gate failed with exit code $LASTEXITCODE."
        }
    }
    catch {
        $status = "failed"
        $errorMessage = $_.Exception.Message
        if (-not $ContinueOnError) {
            throw
        }
    }
    Save-StateSnapshot -Label "after-$ModeId"
    $stateAfter = Get-StateSnapshotSummary -Label "after-$ModeId"
    $stateIssues = Get-StateIssueSummary -Before $stateBefore -After $stateAfter

    $results.Add([pscustomobject]@{
            mode = $ModeId
            architecture = $Architecture
            status = $status
            error = $errorMessage
            runtimeProfile = if ($BrokerSourceMode) { "makepad H.264 $BrokerSourceMode" } else { "makepad direct camera" }
            artifactRoot = $modeRoot
            latestRun = $modeRoot
            stateBefore = $stateBefore
            stateAfter = $stateAfter
            stateIssues = $stateIssues
        })
}

foreach ($modeId in $Mode) {
    Stop-LaneAppsBeforeMode -ModeId $modeId
    Restart-BrokerBeforeMode -ModeId $modeId
    switch ($modeId) {
        "vulkan-hwb-direct-camera2-raw" {
            $offsetYUv = Resolve-ModeProjectionAreaOffsetYUv `
                -RendererValue $VulkanProjectionAreaOffsetYUv `
                -ModeValue $VulkanDirectProjectionAreaOffsetYUv
            $projectionScale = Resolve-VulkanCameraProjectionScale -ModeValue $VulkanDirectCameraProjectionScale
            $xrRenderScale = Resolve-VulkanXrRenderScale -ModeValue $VulkanDirectXrRenderScale
            $projectionAreaScaleUv = Resolve-VulkanProjectionAreaScaleUv -ModeValue $VulkanDirectProjectionAreaScaleUv
            $previewFovY = Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraPreviewFovYDegrees -ModeValue $VulkanDirectCameraPreviewFovYDegrees
            $rawOverlayOverscan = Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraRawOverlayOverscan -ModeValue $VulkanDirectCameraRawOverlayOverscan
            $fullViewOverlayOverscan = Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraFullViewOverlayOverscan -ModeValue $VulkanDirectCameraFullViewOverlayOverscan
            Invoke-QuestProfileMode `
                -ModeId $modeId `
                -Architecture "Camera2 -> ImageReader PRIVATE / HardwareBuffer -> Vulkan/OpenXR raw projection" `
                -Catalog $compositeCatalog `
                -AppId "rusty-xr-quest-composite-layer" `
                -DeviceProfile "xr-composite-comparison-level-5" `
                -RuntimeProfile "camera-stereo-gpu-composite-full-feed-alignment" `
                -Apk $CompositeApk `
                -InstallKey "composite" `
                -Override (Join-OverrideValues -Values @("rustyxr.cameraTargetFps=50", (Get-VulkanProjectionBorderOverride -OffsetYUv $offsetYUv -CameraProjectionScale $projectionScale -XrRenderScale $xrRenderScale -ProjectionAreaScaleUv $projectionAreaScaleUv -CameraPreviewFovYDegrees $previewFovY -CameraRawOverlayOverscan $rawOverlayOverscan -CameraFullViewOverlayOverscan $fullViewOverlayOverscan)))
        }
        "vulkan-hwb-broker-h264-raw" {
            $sourceLabel = if ($BrokerH264SourceMode -eq "broker-synthetic") { "Broker synthetic H.264" } else { "Broker Camera2 -> H.264" }
            $offsetYUv = Resolve-BrokerModeProjectionAreaOffsetYUv `
                -RendererValue $VulkanProjectionAreaOffsetYUv `
                -ModeValue $VulkanBrokerProjectionAreaOffsetYUv `
                -FullFrameRendererValue $VulkanFullFrameBrokerProjectionAreaOffsetYUv
            $projectionScale = Resolve-VulkanCameraProjectionScale -ModeValue $VulkanBrokerCameraProjectionScale
            $xrRenderScale = Resolve-VulkanXrRenderScale -ModeValue $VulkanBrokerXrRenderScale
            $projectionAreaScaleUv = Resolve-VulkanProjectionAreaScaleUv -ModeValue $VulkanBrokerProjectionAreaScaleUv
            $previewFovY = Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraPreviewFovYDegrees -ModeValue $VulkanBrokerCameraPreviewFovYDegrees
            $rawOverlayOverscan = Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraRawOverlayOverscan -ModeValue $VulkanBrokerCameraRawOverlayOverscan
            $fullViewOverlayOverscan = Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraFullViewOverlayOverscan -ModeValue $VulkanBrokerCameraFullViewOverlayOverscan
            Invoke-QuestProfileMode `
                -ModeId $modeId `
                -Architecture "$sourceLabel -> MediaCodec HardwareBuffer -> Vulkan/OpenXR raw projection" `
                -Catalog $compositeCatalog `
                -AppId "rusty-xr-quest-composite-layer" `
                -DeviceProfile "xr-composite-comparison-level-5" `
                -RuntimeProfile "broker-h264-stereo-live-openxr-projection-full-feed-alignment" `
                -Apk $CompositeApk `
                -InstallKey "composite" `
                -Override (Join-OverrideValues -Values @((Get-BrokerH264Override), (Get-VulkanProjectionBorderOverride -OffsetYUv $offsetYUv -CameraProjectionScale $projectionScale -XrRenderScale $xrRenderScale -ProjectionAreaScaleUv $projectionAreaScaleUv -CameraPreviewFovYDegrees $previewFovY -CameraRawOverlayOverscan $rawOverlayOverscan -CameraFullViewOverlayOverscan $fullViewOverlayOverscan)))
        }
        "gles-oes-direct-camera2-raw" {
            $offsetYUv = Resolve-ModeProjectionAreaOffsetYUv `
                -RendererValue $GlesProjectionAreaOffsetYUv `
                -ModeValue $GlesDirectProjectionAreaOffsetYUv
            $scaleUv = Resolve-ModeProjectionAreaScaleUv `
                -RendererValue $GlesProjectionAreaScaleUv `
                -ModeValue $GlesDirectProjectionAreaScaleUv
            Invoke-QuestProfileMode `
                -ModeId $modeId `
                -Architecture "Camera2 -> SurfaceTexture / GL_TEXTURE_EXTERNAL_OES -> OpenGL ES/OpenXR raw projection" `
                -Catalog $glesCatalog `
                -AppId "rusty-xr-quest-gl-openxr-video-stack" `
                -DeviceProfile "gles-openxr-comparison-level-5" `
                -RuntimeProfile "gles-direct-camera2-oes-projection" `
                -Apk $GlesApk `
                -InstallKey "gles" `
                -Override (Get-GlesProjectionBorderOverride -OffsetYUv $offsetYUv -ScaleUv $scaleUv)
        }
        "gles-oes-broker-h264-raw" {
            $sourceLabel = if ($BrokerH264SourceMode -eq "broker-synthetic") { "Broker synthetic H.264" } else { "Broker Camera2 -> H.264" }
            $runtimeProfile = if ($BrokerH264SourceMode -eq "broker-synthetic") { "gles-broker-synthetic-h264-oes-projection" } else { "gles-broker-camera-h264-oes-projection" }
            $offsetYUv = Resolve-BrokerModeProjectionAreaOffsetYUv `
                -RendererValue $GlesProjectionAreaOffsetYUv `
                -ModeValue $GlesBrokerProjectionAreaOffsetYUv `
                -FullFrameRendererValue $GlesFullFrameBrokerProjectionAreaOffsetYUv
            $scaleUv = Resolve-ModeProjectionAreaScaleUv `
                -RendererValue $GlesProjectionAreaScaleUv `
                -ModeValue $GlesBrokerProjectionAreaScaleUv
            Invoke-QuestProfileMode `
                -ModeId $modeId `
                -Architecture "$sourceLabel -> MediaCodec SurfaceTexture/OES -> OpenGL ES/OpenXR raw projection" `
                -Catalog $glesCatalog `
                -AppId "rusty-xr-quest-gl-openxr-video-stack" `
                -DeviceProfile "gles-openxr-comparison-level-5" `
                -RuntimeProfile $runtimeProfile `
                -Apk $GlesApk `
                -InstallKey "gles" `
                -Override (Join-OverrideValues -Values @((Get-BrokerH264Override), (Get-GlesProjectionBorderOverride -OffsetYUv $offsetYUv -ScaleUv $scaleUv)))
        }
        "makepad-cpuyuv-direct-camera2-raw" {
            $offsetYUv = Resolve-ModeProjectionAreaOffsetYUv `
                -RendererValue $MakepadProjectionAreaOffsetYUv `
                -ModeValue $MakepadDirectProjectionAreaOffsetYUv
            $scaleUv = Resolve-ModeProjectionAreaScaleUv `
                -RendererValue $MakepadProjectionAreaScaleUv `
                -ModeValue $MakepadDirectProjectionAreaScaleUv
            $scaleX = Resolve-MakepadProjectionAreaScaleAxis `
                -RendererValue $MakepadProjectionAreaScaleX `
                -ModeValue $MakepadDirectProjectionAreaScaleX `
                -FallbackScaleUv $scaleUv
            $scaleY = Resolve-MakepadProjectionAreaScaleAxis `
                -RendererValue $MakepadProjectionAreaScaleY `
                -ModeValue $MakepadDirectProjectionAreaScaleY `
                -FallbackScaleUv $scaleUv
            $projectionScale = Resolve-MakepadProjectionScale -ModeValue $MakepadDirectProjectionScale
            $xrRenderScale = Resolve-MakepadXrRenderScale -ModeValue $MakepadDirectXrRenderScale
            Invoke-MakepadMode `
                -ModeId $modeId `
                -Architecture "Camera2 -> CPU YUV planes -> Makepad textures/OpenXR raw projection" `
                -OffsetYUv $offsetYUv `
                -ScaleUv $scaleUv `
                -ScaleX $scaleX `
                -ScaleY $scaleY `
                -ProjectionScale $projectionScale `
                -XrRenderScale $xrRenderScale
        }
        "makepad-cpuyuv-broker-h264-raw" {
            $sourceLabel = if ($BrokerH264SourceMode -eq "broker-synthetic") { "Broker synthetic H.264" } else { "Broker Camera2 -> H.264" }
            $offsetYUv = Resolve-BrokerModeProjectionAreaOffsetYUv `
                -RendererValue $MakepadProjectionAreaOffsetYUv `
                -ModeValue $MakepadBrokerProjectionAreaOffsetYUv `
                -FullFrameRendererValue $MakepadFullFrameBrokerProjectionAreaOffsetYUv
            $scaleUv = Resolve-ModeProjectionAreaScaleUv `
                -RendererValue $MakepadProjectionAreaScaleUv `
                -ModeValue $MakepadBrokerProjectionAreaScaleUv
            $scaleX = Resolve-MakepadProjectionAreaScaleAxis `
                -RendererValue $MakepadProjectionAreaScaleX `
                -ModeValue $MakepadBrokerProjectionAreaScaleX `
                -FallbackScaleUv $scaleUv
            $scaleY = Resolve-MakepadProjectionAreaScaleAxis `
                -RendererValue $MakepadProjectionAreaScaleY `
                -ModeValue $MakepadBrokerProjectionAreaScaleY `
                -FallbackScaleUv $scaleUv
            $projectionScale = Resolve-MakepadProjectionScale -ModeValue $MakepadBrokerProjectionScale
            $xrRenderScale = Resolve-MakepadXrRenderScale -ModeValue $MakepadBrokerXrRenderScale
            Invoke-MakepadMode `
                -ModeId $modeId `
                -Architecture "$sourceLabel -> MediaCodec CPU YUV planes -> Makepad textures/OpenXR raw projection" `
                -BrokerSourceMode $BrokerH264SourceMode `
                -OffsetYUv $offsetYUv `
                -ScaleUv $scaleUv `
                -ScaleX $scaleX `
                -ScaleY $scaleY `
                -ProjectionScale $projectionScale `
                -XrRenderScale $xrRenderScale
        }
    }
}

Save-StateSnapshot -Label "final"
if ($RestoreStayAwakeGuard) {
    Restore-StayAwakeGuard -GuardState $stayAwakeGuardState
    Save-StateSnapshot -Label "after-stay-awake-restore"
}

$summaryJson = Join-Path $sessionRoot "raw-camera-stack-suite-summary.json"
$results | ConvertTo-Json -Depth 8 | Set-Content -Path $summaryJson -Encoding UTF8

$summaryMd = Join-Path $sessionRoot "raw-camera-stack-suite-summary.md"
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add("# Raw Camera Stack Alignment Suite")
$lines.Add("")
$lines.Add(("- Session: ``{0}``" -f $sessionId))
$lines.Add(("- Border policy: ``{0}``" -f $ProjectionBorderPolicy))
$lines.Add(("- Processing layer: ``{0}``" -f $ProcessingLayer))
$lines.Add(("- Blur radius px: ``{0}``" -f (Format-InvariantDouble -Value $BlurRadiusPx)))
$lines.Add(("- Projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value $ProjectionAreaOffsetYUv)))
$lines.Add(("- Vulkan/HWB projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ProjectionAreaOffsetYUv -RendererValue $VulkanProjectionAreaOffsetYUv))))
$lines.Add(("- GL/OES projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ProjectionAreaOffsetYUv -RendererValue $GlesProjectionAreaOffsetYUv))))
$lines.Add(("- Makepad CPU-YUV projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ProjectionAreaOffsetYUv -RendererValue $MakepadProjectionAreaOffsetYUv))))
$lines.Add(("- Vulkan/HWB direct projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaOffsetYUv -RendererValue $VulkanProjectionAreaOffsetYUv -ModeValue $VulkanDirectProjectionAreaOffsetYUv))))
$lines.Add(("- Vulkan/HWB broker projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaOffsetYUv -RendererValue $VulkanProjectionAreaOffsetYUv -ModeValue $VulkanBrokerProjectionAreaOffsetYUv))))
$lines.Add(("- GL/OES direct projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaOffsetYUv -RendererValue $GlesProjectionAreaOffsetYUv -ModeValue $GlesDirectProjectionAreaOffsetYUv))))
$lines.Add(("- GL/OES broker projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaOffsetYUv -RendererValue $GlesProjectionAreaOffsetYUv -ModeValue $GlesBrokerProjectionAreaOffsetYUv))))
$lines.Add(("- Makepad CPU-YUV direct projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaOffsetYUv -RendererValue $MakepadProjectionAreaOffsetYUv -ModeValue $MakepadDirectProjectionAreaOffsetYUv))))
$lines.Add(("- Makepad CPU-YUV broker projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaOffsetYUv -RendererValue $MakepadProjectionAreaOffsetYUv -ModeValue $MakepadBrokerProjectionAreaOffsetYUv))))
$fullFrameBrokerOffsetLabel = if ([double]::IsNaN($FullFrameBrokerProjectionAreaOffsetYUv)) {
    "not set"
}
else {
    Format-InvariantDouble -Value $FullFrameBrokerProjectionAreaOffsetYUv
}
$lines.Add(("- Full-frame diagnostic broker projection area offset Y UV: ``{0}``" -f $fullFrameBrokerOffsetLabel))
$lines.Add(("- Vulkan/HWB full-frame broker projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-BrokerModeProjectionAreaOffsetYUv -RendererValue $VulkanProjectionAreaOffsetYUv -ModeValue $VulkanBrokerProjectionAreaOffsetYUv -FullFrameRendererValue $VulkanFullFrameBrokerProjectionAreaOffsetYUv))))
$lines.Add(("- GL/OES full-frame broker projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-BrokerModeProjectionAreaOffsetYUv -RendererValue $GlesProjectionAreaOffsetYUv -ModeValue $GlesBrokerProjectionAreaOffsetYUv -FullFrameRendererValue $GlesFullFrameBrokerProjectionAreaOffsetYUv))))
$lines.Add(("- Makepad CPU-YUV full-frame broker projection area offset Y UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-BrokerModeProjectionAreaOffsetYUv -RendererValue $MakepadProjectionAreaOffsetYUv -ModeValue $MakepadBrokerProjectionAreaOffsetYUv -FullFrameRendererValue $MakepadFullFrameBrokerProjectionAreaOffsetYUv))))
$lines.Add(("- Projection area scale UV: ``{0}``" -f (Format-InvariantDouble -Value $ProjectionAreaScaleUv)))
$lines.Add(("- GL/OES direct projection area scale UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaScaleUv -RendererValue $GlesProjectionAreaScaleUv -ModeValue $GlesDirectProjectionAreaScaleUv))))
$lines.Add(("- GL/OES broker projection area scale UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaScaleUv -RendererValue $GlesProjectionAreaScaleUv -ModeValue $GlesBrokerProjectionAreaScaleUv))))
$lines.Add(("- Makepad CPU-YUV direct projection area scale UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaScaleUv -RendererValue $MakepadProjectionAreaScaleUv -ModeValue $MakepadDirectProjectionAreaScaleUv))))
$lines.Add(("- Makepad CPU-YUV broker projection area scale UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-ModeProjectionAreaScaleUv -RendererValue $MakepadProjectionAreaScaleUv -ModeValue $MakepadBrokerProjectionAreaScaleUv))))
$makepadDirectScaleUv = Resolve-ModeProjectionAreaScaleUv -RendererValue $MakepadProjectionAreaScaleUv -ModeValue $MakepadDirectProjectionAreaScaleUv
$makepadBrokerScaleUv = Resolve-ModeProjectionAreaScaleUv -RendererValue $MakepadProjectionAreaScaleUv -ModeValue $MakepadBrokerProjectionAreaScaleUv
$lines.Add(("- Makepad CPU-YUV direct projection area scale X/Y: ``{0}``, ``{1}``" -f (Format-InvariantDouble -Value (Resolve-MakepadProjectionAreaScaleAxis -RendererValue $MakepadProjectionAreaScaleX -ModeValue $MakepadDirectProjectionAreaScaleX -FallbackScaleUv $makepadDirectScaleUv)), (Format-InvariantDouble -Value (Resolve-MakepadProjectionAreaScaleAxis -RendererValue $MakepadProjectionAreaScaleY -ModeValue $MakepadDirectProjectionAreaScaleY -FallbackScaleUv $makepadDirectScaleUv))))
$lines.Add(("- Makepad CPU-YUV broker projection area scale X/Y: ``{0}``, ``{1}``" -f (Format-InvariantDouble -Value (Resolve-MakepadProjectionAreaScaleAxis -RendererValue $MakepadProjectionAreaScaleX -ModeValue $MakepadBrokerProjectionAreaScaleX -FallbackScaleUv $makepadBrokerScaleUv)), (Format-InvariantDouble -Value (Resolve-MakepadProjectionAreaScaleAxis -RendererValue $MakepadProjectionAreaScaleY -ModeValue $MakepadBrokerProjectionAreaScaleY -FallbackScaleUv $makepadBrokerScaleUv))))
$lines.Add(("- Makepad CPU-YUV direct projection scale: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-MakepadProjectionScale -ModeValue $MakepadDirectProjectionScale))))
$lines.Add(("- Makepad CPU-YUV broker projection scale: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-MakepadProjectionScale -ModeValue $MakepadBrokerProjectionScale))))
$lines.Add(("- Makepad CPU-YUV direct XR render scale: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-MakepadXrRenderScale -ModeValue $MakepadDirectXrRenderScale))))
$lines.Add(("- Makepad CPU-YUV broker XR render scale: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-MakepadXrRenderScale -ModeValue $MakepadBrokerXrRenderScale))))
$lines.Add(("- Vulkan/HWB camera projection scale default: ``{0}``" -f (Format-InvariantDouble -Value $VulkanCameraProjectionScale)))
$lines.Add(("- Vulkan/HWB direct camera projection scale: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-VulkanCameraProjectionScale -ModeValue $VulkanDirectCameraProjectionScale))))
$lines.Add(("- Vulkan/HWB broker camera projection scale: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-VulkanCameraProjectionScale -ModeValue $VulkanBrokerCameraProjectionScale))))
$lines.Add(("- Vulkan/HWB direct preview FOV Y / raw overscan / full-view overscan: ``{0}``, ``{1}``, ``{2}``" -f (Format-OptionalInvariantDouble -Value (Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraPreviewFovYDegrees -ModeValue $VulkanDirectCameraPreviewFovYDegrees)), (Format-OptionalInvariantDouble -Value (Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraRawOverlayOverscan -ModeValue $VulkanDirectCameraRawOverlayOverscan)), (Format-OptionalInvariantDouble -Value (Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraFullViewOverlayOverscan -ModeValue $VulkanDirectCameraFullViewOverlayOverscan))))
$lines.Add(("- Vulkan/HWB broker preview FOV Y / raw overscan / full-view overscan: ``{0}``, ``{1}``, ``{2}``" -f (Format-OptionalInvariantDouble -Value (Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraPreviewFovYDegrees -ModeValue $VulkanBrokerCameraPreviewFovYDegrees)), (Format-OptionalInvariantDouble -Value (Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraRawOverlayOverscan -ModeValue $VulkanBrokerCameraRawOverlayOverscan)), (Format-OptionalInvariantDouble -Value (Resolve-VulkanOptionalDouble -RendererValue $VulkanCameraFullViewOverlayOverscan -ModeValue $VulkanBrokerCameraFullViewOverlayOverscan))))
$lines.Add(("- Vulkan/HWB direct projection area scale UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-VulkanProjectionAreaScaleUv -ModeValue $VulkanDirectProjectionAreaScaleUv))))
$lines.Add(("- Vulkan/HWB broker projection area scale UV: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-VulkanProjectionAreaScaleUv -ModeValue $VulkanBrokerProjectionAreaScaleUv))))
$lines.Add(("- Vulkan/HWB direct XR render scale: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-VulkanXrRenderScale -ModeValue $VulkanDirectXrRenderScale))))
$lines.Add(("- Vulkan/HWB broker XR render scale: ``{0}``" -f (Format-InvariantDouble -Value (Resolve-VulkanXrRenderScale -ModeValue $VulkanBrokerXrRenderScale))))
$lines.Add(("- Vulkan/HWB projection area mask radius/corner UV: ``{0}``, ``{1}``, ``{2}``" -f (Format-InvariantDouble -Value $VulkanProjectionAreaRadiusXUv), (Format-InvariantDouble -Value $VulkanProjectionAreaRadiusYUv), (Format-InvariantDouble -Value $VulkanProjectionAreaCornerRadiusUv)))
$lines.Add(("- GL/OES projection area mask radius UV: ``{0}``, ``{1}``" -f (Format-InvariantDouble -Value $GlesProjectionAreaRadiusXUv), (Format-InvariantDouble -Value $GlesProjectionAreaRadiusYUv)))
$lines.Add(("- GL/OES projection area corner radius UV: ``{0}``" -f (Format-InvariantDouble -Value $GlesProjectionAreaCornerRadiusUv)))
$lines.Add(("- Makepad CPU-YUV projection area mask radius UV: ``{0}``, ``{1}``" -f (Format-InvariantDouble -Value $MakepadProjectionAreaRadiusXUv), (Format-InvariantDouble -Value $MakepadProjectionAreaRadiusYUv)))
$lines.Add(("- Makepad CPU-YUV projection area corner radius UV: ``{0}``" -f (Format-InvariantDouble -Value $MakepadProjectionAreaCornerRadiusUv)))
$lines.Add(("- Projection area opacity: ``{0}``" -f (Format-InvariantDouble -Value $ProjectionAreaOpacity)))
$lines.Add(("- Projection border opacity: ``{0}``" -f (Format-InvariantDouble -Value $ProjectionBorderOpacity)))
$lines.Add(("- GL/OES camera color matrix: ``{0}``" -f $GlesCameraColorMatrix))
$lines.Add(("- GL/OES camera color offset: ``{0}``" -f $GlesCameraColorOffset))
$lines.Add(("- GL/OES camera color contrast/brightness/saturation: ``{0}``, ``{1}``, ``{2}``" -f (Format-InvariantDouble -Value $GlesCameraColorContrast), (Format-InvariantDouble -Value $GlesCameraColorBrightness), (Format-InvariantDouble -Value $GlesCameraColorSaturation)))
$lines.Add(("- Native passthrough underlay requested: ``{0}``" -f [bool]$EnableNativePassthroughUnderlay))
$lines.Add(("- Vulkan/HWB border override: ``{0}``" -f (Get-VulkanProjectionBorderOverride)))
$lines.Add(("- GL/OES border override: ``{0}``" -f (Get-GlesProjectionBorderOverride)))
$lines.Add(("- Warmup seconds: ``{0}``" -f $WarmupSeconds))
$lines.Add(("- Sample seconds: ``{0}``" -f $SampleSeconds))
$lines.Add(("- Freshness frames: ``{0}``" -f $FreshnessFrames))
$lines.Add(("- Broker H.264 source mode: ``{0}``" -f $BrokerH264SourceMode))
$lines.Add(("- Broker H.264 synthetic pattern: ``{0}``" -f $BrokerH264SyntheticPattern))
$lines.Add(("- Broker H.264 synthetic projection profile: ``{0}``" -f $BrokerH264SyntheticProjectionProfile))
$lines.Add(("- Broker H.264 stream ports: left ``{0}``, right ``{1}``" -f $BrokerH264LeftStreamPort, $BrokerH264RightStreamPort))
$lines.Add(("- Broker H.264 source shape: ``{0}x{1}``, bitrate ``{2}``, requested FPS ``{3}``, capture ms ``{4}``, max packets ``{5}``" -f $BrokerH264Width, $BrokerH264Height, $BrokerH264BitrateBps, $BrokerH264FrameRateHz, $BrokerH264CaptureMs, $BrokerH264MaxPackets))
$lines.Add(("- Broker camera IDs: left ``{0}``, right ``{1}``" -f $BrokerH264LeftCameraId, $BrokerH264RightCameraId))
$lines.Add(("- Lane app force-stop before each mode: ``{0}``" -f (-not [bool]$SkipLaneAppForceStop)))
if (-not $SkipLaneAppForceStop) {
    $lines.Add(("- Lane app force-stop settle seconds: ``{0}``" -f $LaneAppForceStopSettleSeconds))
    $lines.Add("- Lane app force-stop logs: ``lane-app-force-stops/``")
}
$lines.Add(("- Restart broker before broker modes: ``{0}``" -f [bool]$RestartBrokerBeforeBrokerModes))
if ($RestartBrokerBeforeBrokerModes) {
    $lines.Add(("- Broker restart target: ``{0}/{1}``, settle seconds ``{2}``" -f $BrokerPackageName, $BrokerActivityName, $BrokerRestartSettleSeconds))
    $lines.Add("- Broker restart snapshots: ``broker-restarts/``")
}
$lines.Add("- Passive state snapshots: ``state-snapshots/``")
if ($EnableStayAwakeGuard) {
    $lines.Add("- Stay-awake guard: enabled with ``svc power stayon true``")
    if ($stayAwakeGuardState) {
        $lines.Add(("- Stay-awake setting: ``{0}`` -> ``{1}``" -f $stayAwakeGuardState.beforeStayOnWhilePluggedIn, $stayAwakeGuardState.afterStayOnWhilePluggedIn))
    }
}
else {
    $lines.Add("- Stay-awake guard: not changed by this suite run")
}
if ($RestoreStayAwakeGuard) {
    $lines.Add("- Stay-awake restore: requested after final snapshot")
}
$lines.Add("")
$lines.Add("| Mode | Status | Architecture | Artifact |")
$lines.Add("| --- | --- | --- | --- |")
foreach ($result in $results) {
    $artifact = if ($result.latestRun) { $result.latestRun } else { $result.artifactRoot }
    $lines.Add(('| `{0}` | `{1}` | {2} | `{3}` |' -f $result.mode, $result.status, $result.architecture, $artifact))
    if ($result.error) {
        $lines.Add(('| `{0}` | error | {1} | `{2}` |' -f $result.mode, $result.error.Replace('|', '/'), $artifact))
    }
}
$stateIssueRows = @($results | Where-Object { $_.stateIssues })
if ($stateIssueRows.Count -gt 0) {
    $lines.Add("")
    $lines.Add("## State Transition Audit")
    $lines.Add("")
    $lines.Add("If wakefulness or VR power state changes during a mode, treat later camera evidence as bracketed until readiness is re-proven.")
    $lines.Add("")
    $lines.Add("| Mode | Before | After | Issues |")
    $lines.Add("| --- | --- | --- | --- |")
    foreach ($result in $stateIssueRows) {
        $beforeState = if ($result.stateBefore.vrState) { $result.stateBefore.vrState } else { "unknown" }
        $afterState = if ($result.stateAfter.vrState) { $result.stateAfter.vrState } else { "unknown" }
        $beforeWake = if ($result.stateBefore.wakefulness) { $result.stateBefore.wakefulness } else { "unknown" }
        $afterWake = if ($result.stateAfter.wakefulness) { $result.stateAfter.wakefulness } else { "unknown" }
        $lines.Add(('| `{0}` | `{1}` / `{2}` | `{3}` / `{4}` | {5} |' -f $result.mode, $beforeWake, $beforeState, $afterWake, $afterState, $result.stateIssues.Replace('|', '/')))
    }
}
$lines.Add("")
$lines.Add("Use diagnostic-split or solid-red borders for image-derived footprint work and passthrough-underlay borders for operator alignment against native passthrough.")
$lines | Set-Content -Path $summaryMd -Encoding UTF8

Write-Host "Raw camera stack suite summary:"
Write-Host $summaryMd
