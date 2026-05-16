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
    [string]$BrokerH264LeftCameraId = "50",
    [string]$BrokerH264RightCameraId = "51",
    [ValidateSet("solid-red", "passthrough-underlay")]
    [string]$ProjectionBorderPolicy = "solid-red",
    [switch]$EnableStayAwakeGuard,
    [switch]$RestoreStayAwakeGuard,
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

function Get-VulkanProjectionBorderOverride {
    if ($ProjectionBorderPolicy -eq "passthrough-underlay") {
        return "rustyxr.cameraPipelinePreset=raw-projection-underlay-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-underlay,rustyxr.openxrPassthroughProbe=underlay"
    }
    return "rustyxr.cameraPipelinePreset=raw-projection-solid-red-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-solid-red,rustyxr.openxrPassthroughProbe=off"
}

function Get-GlesProjectionBorderOverride {
    return "rustyxr.projectionBorderPolicy=$ProjectionBorderPolicy"
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
        })
}

function Invoke-MakepadMode {
    param(
        [string]$ModeId,
        [string]$Architecture,
        [switch]$UseBrokerH264Camera
    )

    if (-not $MakepadApk -or -not $MakepadPackageName -or -not $MakepadLauncherActivity -or -not $MakepadXrActivity) {
        throw "Makepad modes require -MakepadApk, -MakepadPackageName, -MakepadLauncherActivity, and -MakepadXrActivity."
    }

    $modeRoot = Join-Path $sessionRoot $ModeId
    New-Item -ItemType Directory -Force -Path $modeRoot | Out-Null
    Save-StateSnapshot -Label "before-$ModeId"

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

    if ($UseBrokerH264Camera) {
        $argList.Add("-UseBrokerH264Camera")
        $argList.Add("-BrokerH264LeftCameraId")
        $argList.Add($BrokerH264LeftCameraId)
        $argList.Add("-BrokerH264RightCameraId")
        $argList.Add($BrokerH264RightCameraId)
        $argList.Add("-BrokerH264CaptureMs")
        $argList.Add("0")
        $argList.Add("-BrokerH264MaxPackets")
        $argList.Add("0")
        $argList.Add("-BrokerH264FrameRateHz")
        $argList.Add("50")
        $argList.Add("-BrokerH264Width")
        $argList.Add("1280")
        $argList.Add("-BrokerH264Height")
        $argList.Add("1280")
        $argList.Add("-BrokerH264BitrateBps")
        $argList.Add("6000000")
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

    $results.Add([pscustomobject]@{
            mode = $ModeId
            architecture = $Architecture
            status = $status
            error = $errorMessage
            runtimeProfile = if ($UseBrokerH264Camera) { "makepad broker H.264 camera" } else { "makepad direct camera" }
            artifactRoot = $modeRoot
            latestRun = $modeRoot
        })
}

foreach ($modeId in $Mode) {
    switch ($modeId) {
        "vulkan-hwb-direct-camera2-raw" {
            Invoke-QuestProfileMode `
                -ModeId $modeId `
                -Architecture "Camera2 -> ImageReader PRIVATE / HardwareBuffer -> Vulkan/OpenXR raw projection" `
                -Catalog $compositeCatalog `
                -AppId "rusty-xr-quest-composite-layer" `
                -DeviceProfile "xr-composite-comparison-level-5" `
                -RuntimeProfile "camera-stereo-gpu-composite-fast075" `
                -Apk $CompositeApk `
                -InstallKey "composite" `
                -Override (Join-OverrideValues -Values @("rustyxr.cameraTargetFps=50", (Get-VulkanProjectionBorderOverride)))
        }
        "vulkan-hwb-broker-h264-raw" {
            Invoke-QuestProfileMode `
                -ModeId $modeId `
                -Architecture "Broker Camera2 -> H.264 -> MediaCodec HardwareBuffer -> Vulkan/OpenXR raw projection" `
                -Catalog $compositeCatalog `
                -AppId "rusty-xr-quest-composite-layer" `
                -DeviceProfile "xr-composite-comparison-level-5" `
                -RuntimeProfile "broker-h264-stereo-live-openxr-projection-fast075-probe" `
                -Apk $CompositeApk `
                -InstallKey "composite" `
                -Override (Join-OverrideValues -Values @("rustyxr.brokerH264CaptureMs=0,rustyxr.brokerH264MaxPackets=0,rustyxr.brokerH264FrameRateHz=50,rustyxr.brokerH264Width=1280,rustyxr.brokerH264Height=1280,rustyxr.brokerH264BitrateBps=6000000,rustyxr.brokerH264LiveStream=true,rustyxr.brokerH264LiveDecode=true", (Get-VulkanProjectionBorderOverride)))
        }
        "gles-oes-direct-camera2-raw" {
            Invoke-QuestProfileMode `
                -ModeId $modeId `
                -Architecture "Camera2 -> SurfaceTexture / GL_TEXTURE_EXTERNAL_OES -> OpenGL ES/OpenXR raw projection" `
                -Catalog $glesCatalog `
                -AppId "rusty-xr-quest-gl-openxr-video-stack" `
                -DeviceProfile "gles-openxr-comparison-level-5" `
                -RuntimeProfile "gles-direct-camera2-oes-projection" `
                -Apk $GlesApk `
                -InstallKey "gles" `
                -Override (Get-GlesProjectionBorderOverride)
        }
        "gles-oes-broker-h264-raw" {
            Invoke-QuestProfileMode `
                -ModeId $modeId `
                -Architecture "Broker Camera2 -> H.264 -> MediaCodec SurfaceTexture/OES -> OpenGL ES/OpenXR raw projection" `
                -Catalog $glesCatalog `
                -AppId "rusty-xr-quest-gl-openxr-video-stack" `
                -DeviceProfile "gles-openxr-comparison-level-5" `
                -RuntimeProfile "gles-broker-camera-h264-oes-projection" `
                -Apk $GlesApk `
                -InstallKey "gles" `
                -Override (Join-OverrideValues -Values @("rustyxr.brokerH264CaptureMs=0,rustyxr.brokerH264MaxPackets=0,rustyxr.brokerH264FrameRateHz=50,rustyxr.brokerH264Width=1280,rustyxr.brokerH264Height=1280,rustyxr.brokerH264BitrateBps=6000000,rustyxr.brokerH264LiveStream=true,rustyxr.brokerH264LiveDecode=true", (Get-GlesProjectionBorderOverride)))
        }
        "makepad-cpuyuv-direct-camera2-raw" {
            Invoke-MakepadMode `
                -ModeId $modeId `
                -Architecture "Camera2 -> CPU YUV planes -> Makepad textures/OpenXR raw projection"
        }
        "makepad-cpuyuv-broker-h264-raw" {
            Invoke-MakepadMode `
                -ModeId $modeId `
                -Architecture "Broker Camera2 -> H.264 -> MediaCodec CPU YUV planes -> Makepad textures/OpenXR raw projection" `
                -UseBrokerH264Camera
        }
    }
}

Save-StateSnapshot -Label "final"
if ($RestoreStayAwakeGuard) {
    Restore-StayAwakeGuard -GuardState $stayAwakeGuardState
    Save-StateSnapshot -Label "after-stay-awake-restore"
}

$summaryJson = Join-Path $sessionRoot "raw-camera-stack-suite-summary.json"
$results | ConvertTo-Json -Depth 4 | Set-Content -Path $summaryJson -Encoding UTF8

$summaryMd = Join-Path $sessionRoot "raw-camera-stack-suite-summary.md"
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add("# Raw Camera Stack Alignment Suite")
$lines.Add("")
$lines.Add(("- Session: ``{0}``" -f $sessionId))
$lines.Add(("- Border policy: ``{0}``" -f $ProjectionBorderPolicy))
$lines.Add(("- Vulkan/HWB border override: ``{0}``" -f (Get-VulkanProjectionBorderOverride)))
$lines.Add(("- GL/OES border override: ``{0}``" -f (Get-GlesProjectionBorderOverride)))
$lines.Add(("- Warmup seconds: ``{0}``" -f $WarmupSeconds))
$lines.Add(("- Sample seconds: ``{0}``" -f $SampleSeconds))
$lines.Add(("- Freshness frames: ``{0}``" -f $FreshnessFrames))
$lines.Add(("- Broker camera IDs: left ``{0}``, right ``{1}``" -f $BrokerH264LeftCameraId, $BrokerH264RightCameraId))
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
$lines.Add("")
$lines.Add("Use solid-red borders for image-derived footprint work and passthrough-underlay borders for operator alignment against native passthrough.")
$lines | Set-Content -Path $summaryMd -Encoding UTF8

Write-Host "Raw camera stack suite summary:"
Write-Host $summaryMd
