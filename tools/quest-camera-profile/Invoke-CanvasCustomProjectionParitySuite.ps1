param(
    [string]$Serial = "",
    [string]$Adb = "adb",
    [string]$Npx = "npx",
    [string]$RunRoot = "artifacts\canvas-custom-projection-parity-suite",
    [ValidateSet("custom", "fast-visual", "full-evidence")]
    [string]$EvidenceMode = "custom",
    [string]$HwbApk = "examples\quest-composite-layer-apk\build\outputs\rusty-xr-quest-composite-layer-debug.apk",
    [string]$GlesApk = "examples\quest-gl-openxr-video-stack-apk\build\outputs\rusty-xr-quest-gl-openxr-video-stack-debug.apk",
    [string]$MakepadApk = "examples\makepad-camera-shell\target\android\makepad-android-apk\rusty_xr_makepad_camera_shell\apk\rustyx_rmakepadcamera.apk",
    [string]$MakepadPackageName = "io.github.mesmerprism.rustyxr.makepad.camera",
    [int]$WarmupSeconds = 12,
    [int]$MakepadStartupTimeoutSeconds = 60,
    [int]$MakepadSampleSeconds = 32,
    [int]$MakepadPostRunSettleSeconds = 8,
    [int]$MediaProjectionPort = 8787,
    [int]$MediaProjectionMaxFrames = 0,
    [int]$MediaProjectionDrainMs = 1500,
    [switch]$SkipMediaProjection,
    [ValidateSet("fast-adb", "hzdb")]
    [string]$HeadsetCaptureProvider = "fast-adb",
    [switch]$SkipAnalyzer,
    [switch]$FailOnAnalyzerIssue,
    [ValidateSet("direct-camera", "broker-camera", "broker-synthetic")]
    [string]$SourceMode = "direct-camera",
    [ValidateSet("passthrough-underlay", "solid-red")]
    [string]$ProjectionBorderPolicy = "passthrough-underlay",
    [ValidateSet("all", "hwb", "oes", "makepad")]
    [string[]]$LaneFilter = @("all"),
    [ValidateSet("raw", "blur")]
    [string]$ProcessingLayer = "raw",
    [double]$BlurRadiusPx = 2.0,
    [double]$ProjectionAreaOpacity = 1.0,
    [double]$ProjectionBorderOpacity = 1.0,
    [switch]$BoundedCanvasProjectionArea,
    [switch]$UseResolvedProjectionRuntime = $true,
    [ValidateSet("skip", "warn", "required")]
    [string]$ProjectionRuntimeReadback = "warn",
    [string]$BrokerPackageName = "com.example.rustyxr.broker",
    [string]$BrokerActivityName = ".BrokerStartActivity",
    [int]$BrokerRestartSettleSeconds = 3,
    [string]$BrokerH264LeftCameraId = "50",
    [string]$BrokerH264RightCameraId = "51",
    [int]$BrokerH264LeftStreamPort = 8879,
    [int]$BrokerH264RightStreamPort = 8880,
    [int]$BrokerH264FrameRateHz = 50,
    [int]$BrokerH264BitrateBps = 6000000,
    [ValidateSet("diagnostic-grid", "motion-bar", "checkerboard", "luma-ramp")]
    [string]$BrokerH264SyntheticPattern = "diagnostic-grid",
    [ValidateSet("head-anchored-virtual-camera", "camera-matched", "full-frame-diagnostic")]
    [string]$BrokerH264SyntheticProjectionProfile = "camera-matched",
    [switch]$Install
)

$ErrorActionPreference = "Stop"

if ([double]::IsNaN($BlurRadiusPx) -or [double]::IsInfinity($BlurRadiusPx) -or $BlurRadiusPx -lt 0.0 -or $BlurRadiusPx -gt 16.0) {
    throw "BlurRadiusPx must be a finite value from 0.0 to 16.0"
}

switch ($EvidenceMode) {
    "fast-visual" {
        if ($PSBoundParameters.ContainsKey("HeadsetCaptureProvider") -and $HeadsetCaptureProvider -ne "fast-adb") {
            throw "EvidenceMode fast-visual selects HeadsetCaptureProvider fast-adb. Use EvidenceMode custom for a different headset screenshot provider."
        }
        $HeadsetCaptureProvider = "fast-adb"
    }
    "full-evidence" {
        if ($PSBoundParameters.ContainsKey("SkipMediaProjection") -and [bool]$SkipMediaProjection) {
            throw "EvidenceMode full-evidence enables MediaProjection. Use EvidenceMode custom with -SkipMediaProjection for headset-only evidence."
        }
        if ($PSBoundParameters.ContainsKey("SkipAnalyzer") -and [bool]$SkipAnalyzer) {
            throw "EvidenceMode full-evidence enables the analyzer. Use EvidenceMode custom with -SkipAnalyzer for capture-only runs."
        }
        if ($PSBoundParameters.ContainsKey("HeadsetCaptureProvider") -and $HeadsetCaptureProvider -ne "hzdb") {
            throw "EvidenceMode full-evidence requires HeadsetCaptureProvider hzdb. Use EvidenceMode custom for mixed capture settings."
        }
        $SkipMediaProjection = $false
        $SkipAnalyzer = $false
        $HeadsetCaptureProvider = "hzdb"
    }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$runRootPath = if ([System.IO.Path]::IsPathRooted($RunRoot)) {
    $RunRoot
} else {
    Join-Path $repoRoot $RunRoot
}
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$sessionRoot = Join-Path $runRootPath $stamp
$screenshotsRoot = Join-Path $sessionRoot "screenshots"
$profileRunRoot = Join-Path $sessionRoot "profile-runs"
$makepadRunRoot = Join-Path $sessionRoot "makepad-runs"
New-Item -ItemType Directory -Force -Path $screenshotsRoot, $profileRunRoot, $makepadRunRoot | Out-Null

$timingPath = Join-Path $sessionRoot "step-timings.jsonl"
$timingSummaryPath = Join-Path $sessionRoot "step-timing-summary.json"
$script:suiteStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$script:timingRecords = [System.Collections.Generic.List[object]]::new()
$script:boundedFootprintEvidenceRecords = [System.Collections.Generic.List[object]]::new()

$receiver = Join-Path $repoRoot "tools\media-pipeline\frame_receiver.py"
$converter = Join-Path $repoRoot "tools\media-pipeline\Convert-RgbaFrameToPng.py"
$contactSheetBuilder = Join-Path $repoRoot "tools\quest-camera-profile\Build-CanvasCustomParityContactSheet.py"
$screenSpaceAnalyzer = Join-Path $repoRoot "tools\quest-camera-profile\Analyze-RawStackScreenSpace.py"
$artifactValidator = Join-Path $repoRoot "tools\quest-camera-profile\Validate-CanvasCustomParityArtifacts.py"
$profileRunner = Join-Path $repoRoot "tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1"
$makepadRunner = Join-Path $repoRoot "examples\makepad-camera-shell\tools\Invoke-MakepadCameraDeviceGate.ps1"
$projectionRuntimeResolutionEnabledValue = if ($UseResolvedProjectionRuntime) { "true" } else { "false" }
$effectiveProjectionRuntimeReadback = if ($ProjectionRuntimeReadback -eq "warn" -and $UseResolvedProjectionRuntime) { "required" } else { $ProjectionRuntimeReadback }

function Format-LaunchFloat {
    param([double]$Value)
    return $Value.ToString("0.0#####", [System.Globalization.CultureInfo]::InvariantCulture)
}

function Add-TimingRecord {
    param([object]$Record)
    $script:timingRecords.Add($Record)
    $Record | ConvertTo-Json -Depth 5 -Compress | Add-Content -Path $timingPath -Encoding UTF8
}

function Invoke-TimedStep {
    param(
        [string]$CaseId,
        [string]$Step,
        [scriptblock]$Action
    )
    $startedAt = Get-Date
    $startedElapsedMs = $script:suiteStopwatch.ElapsedMilliseconds
    $status = "ok"
    $errorMessage = ""
    try {
        & $Action
    } catch {
        $status = "failed"
        $errorMessage = $_.Exception.Message
        throw
    } finally {
        $endedAt = Get-Date
        $endedElapsedMs = $script:suiteStopwatch.ElapsedMilliseconds
        Add-TimingRecord -Record ([ordered]@{
            caseId = $CaseId
            step = $Step
            status = $status
            startedAt = $startedAt.ToString("o")
            endedAt = $endedAt.ToString("o")
            startElapsedMs = $startedElapsedMs
            endElapsedMs = $endedElapsedMs
            durationMs = $endedElapsedMs - $startedElapsedMs
            error = $errorMessage
        })
        Write-Host ("[timing] {0} {1} {2}ms {3}" -f $CaseId, $Step, ($endedElapsedMs - $startedElapsedMs), $status)
    }
}

function Get-TimingRecordValue {
    param(
        [object]$Record,
        [string]$Name
    )
    if ($Record -is [System.Collections.IDictionary]) {
        return $Record[$Name]
    }
    return $Record.$Name
}

function New-TimingSummary {
    $records = @($script:timingRecords)
    $byStep = @(
        $records |
            Group-Object -Property { Get-TimingRecordValue -Record $_ -Name "step" } |
            ForEach-Object {
                $durations = @($_.Group | ForEach-Object { [double](Get-TimingRecordValue -Record $_ -Name "durationMs") })
                $durationStats = $durations | Measure-Object -Sum -Minimum -Maximum -Average
                [ordered]@{
                    step = $_.Name
                    count = $_.Count
                    totalMs = if ($durations.Count -gt 0) { [long]$durationStats.Sum } else { 0 }
                    minMs = if ($durations.Count -gt 0) { [long]$durationStats.Minimum } else { 0 }
                    maxMs = if ($durations.Count -gt 0) { [long]$durationStats.Maximum } else { 0 }
                    avgMs = if ($durations.Count -gt 0) { [Math]::Round([double]$durationStats.Average, 1) } else { 0.0 }
                    failures = @($_.Group | Where-Object { (Get-TimingRecordValue -Record $_ -Name "status") -ne "ok" }).Count
                }
            }
    )
    return [ordered]@{
        schemaVersion = "rusty.xr.canvas-custom-projection-parity-suite.timing.v1"
        totalElapsedMs = $script:suiteStopwatch.ElapsedMilliseconds
        timingJsonl = $timingPath
        records = $records
        byStep = $byStep
    }
}

$surfaceOverrideValues = @(
    "rustyxr.projectionDepthMeters=1.434085",
    "rustyxr.cameraPreviewFovYDegrees=69.763084",
    "rustyxr.cameraPreviewOffsetYMeters=-0.168832",
    "rustyxr.cameraRawOverlayOverscan=1.0",
    ("rustyxr.projectionRuntimeResolutionEnabled={0}" -f $projectionRuntimeResolutionEnabledValue)
)
if ($SkipMediaProjection) {
    $surfaceOverrideValues += "rustyxr.mediaProjection=false"
} else {
    $surfaceOverrideValues += @(
        "rustyxr.mediaProjection=true",
        ("rustyxr.mediaProjectionPort={0}" -f $MediaProjectionPort),
        "rustyxr.mediaProjectionWidth=512",
        "rustyxr.mediaProjectionHeight=288"
    )
}
$surfaceOverride = $surfaceOverrideValues -join ","

$boundedProjectionAreaOverride = @(
    "rustyxr.projectionAreaOffsetXUv=0.0",
    "rustyxr.projectionAreaOffsetYUv=0.0",
    "rustyxr.projectionAreaScaleUv=1.0",
    "rustyxr.projectionAreaRadiusXUv=0.47",
    "rustyxr.projectionAreaRadiusYUv=0.36",
    "rustyxr.projectionAreaCornerRadiusUv=0.08"
) -join ","

$projectionAreaRadiusXUv = if ($BoundedCanvasProjectionArea) { "0.47" } else { "0.5" }
$projectionAreaRadiusYUv = if ($BoundedCanvasProjectionArea) { "0.36" } else { "0.5" }
$projectionAreaCornerRadiusUv = if ($BoundedCanvasProjectionArea) { "0.08" } else { "0.0" }

$projectionOpacityOverride = @(
    ("rustyxr.projectionAreaOpacity={0}" -f (Format-LaunchFloat -Value $ProjectionAreaOpacity)),
    ("rustyxr.projectionBorderOpacity={0}" -f (Format-LaunchFloat -Value $ProjectionBorderOpacity)),
    ("rustyxr.projectionBorderPolicy={0}" -f $ProjectionBorderPolicy)
) -join ","
$blurRadiusPxText = Format-LaunchFloat -Value $BlurRadiusPx
$processingLayerOverride = @(
    ("rustyxr.processingLayer={0}" -f $ProcessingLayer),
    ("rustyxr.cameraBlurRadiusPx={0}" -f $blurRadiusPxText)
) -join ","
$ExpectedMakepadSourceEyeMapping = "display-left-from-left-source"
$brokerSourceRequested = $SourceMode -eq "broker-camera" -or $SourceMode -eq "broker-synthetic"

Write-Host ("[suite] evidenceMode={0} headsetCaptureProvider={1} mediaProjection={2} analyzer={3} projectionBorderPolicy={4} projectionRuntimeReadback={5}" -f `
    $EvidenceMode,
    $HeadsetCaptureProvider,
    (-not [bool]$SkipMediaProjection),
    (-not [bool]$SkipAnalyzer),
    $ProjectionBorderPolicy,
    $effectiveProjectionRuntimeReadback)

function Test-LaneEnabled {
    param([string]$Lane)
    if ($LaneFilter -contains "all") {
        return $true
    }
    return $LaneFilter -contains $Lane
}

function Get-BrokerH264Override {
    param([string]$ProjectionGeometryProfile)
    $brokerProjectionGeometryProfile = if ($SourceMode -eq "broker-synthetic") {
        $BrokerH264SyntheticProjectionProfile
    } else {
        $ProjectionGeometryProfile
    }
    return @(
        ("rustyxr.brokerH264SourceMode={0}" -f $SourceMode),
        ("rustyxr.brokerH264ProjectionGeometryProfile={0}" -f $brokerProjectionGeometryProfile),
        ("rustyxr.brokerH264SyntheticPattern={0}" -f $BrokerH264SyntheticPattern),
        ("rustyxr.brokerH264SyntheticProjectionProfile={0}" -f $BrokerH264SyntheticProjectionProfile),
        ("rustyxr.brokerH264StreamPort={0}" -f $BrokerH264LeftStreamPort),
        ("rustyxr.brokerH264RightStreamPort={0}" -f $BrokerH264RightStreamPort),
        ("rustyxr.brokerH264LeftCameraId={0}" -f $BrokerH264LeftCameraId),
        ("rustyxr.brokerH264RightCameraId={0}" -f $BrokerH264RightCameraId),
        "rustyxr.brokerH264Width=1280",
        "rustyxr.brokerH264Height=1280",
        "rustyxr.brokerH264CaptureMs=0",
        "rustyxr.brokerH264MaxPackets=0",
        ("rustyxr.brokerH264FrameRateHz={0}" -f $BrokerH264FrameRateHz),
        ("rustyxr.brokerH264BitrateBps={0}" -f $BrokerH264BitrateBps),
        "rustyxr.brokerH264LiveStream=true",
        "rustyxr.brokerH264LiveDecode=true"
    ) -join ","
}

function Get-HwbProjectionStyleOverride {
    param([string]$Mode)
    $passthroughOverride = if ($ProjectionBorderPolicy -eq "passthrough-underlay" -or
        $ProjectionAreaOpacity -lt 1.0 -or
        $ProjectionBorderOpacity -lt 1.0) {
        "rustyxr.openxrPassthroughProbe=underlay"
    }
    else {
        "rustyxr.openxrPassthroughProbe=off"
    }
    return "rustyxr.cameraPipelinePreset=raw-projection-unorm,rustyxr.cameraProjectionEffectMode=raw-projection,$passthroughOverride,$projectionOpacityOverride,$processingLayerOverride"
}

function Get-GlesProjectionStyleOverride {
    return "$projectionOpacityOverride,$processingLayerOverride"
}

function Get-MakepadNativePassthroughRequested {
    return $ProjectionBorderPolicy -eq "passthrough-underlay" -or
        $ProjectionAreaOpacity -lt 1.0 -or
        $ProjectionBorderOpacity -lt 1.0
}

function Join-OverrideValues {
    param([string[]]$Values)
    return (($Values | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ",")
}

function Get-CatalogPackageName {
    param([string]$Catalog)
    $catalogPath = Resolve-RepoPath $Catalog
    $catalogJson = Get-Content -Raw -Path $catalogPath | ConvertFrom-Json
    if ($catalogJson.apps -and $catalogJson.apps.Count -gt 0) {
        return [string]$catalogJson.apps[0].packageName
    }
    return ""
}

function Resolve-RepoPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function ConvertTo-WindowsLongPath {
    param([string]$Path)
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    if ($fullPath.StartsWith("\\?\")) {
        return $fullPath
    }
    if ($fullPath.StartsWith("\\")) {
        return "\\?\UNC\" + $fullPath.Substring(2)
    }
    return "\\?\" + $fullPath
}

function Invoke-Adb {
    param([string[]]$Arguments)
    if ($Serial) {
        & $Adb -s $Serial @Arguments
    } else {
        & $Adb @Arguments
    }
    if ($LASTEXITCODE -ne 0) {
        throw "adb failed: $($Arguments -join ' ')"
    }
}

function Get-LocalTcpListeners {
    param([int]$Port)
    @(
        Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
            ForEach-Object {
                $processName = ""
                try {
                    $processName = (Get-Process -Id $_.OwningProcess -ErrorAction Stop).ProcessName
                } catch {
                    $processName = "unknown"
                }
                [pscustomobject]@{
                    LocalAddress = $_.LocalAddress
                    LocalPort = $_.LocalPort
                    OwningProcess = $_.OwningProcess
                    ProcessName = $processName
                }
            }
    )
}

function Format-LocalTcpListeners {
    param([object[]]$Listeners)
    if ($null -eq $Listeners -or $Listeners.Count -eq 0) {
        return "none"
    }
    return (($Listeners | ForEach-Object {
        "{0}:{1} pid={2} process={3}" -f $_.LocalAddress, $_.LocalPort, $_.OwningProcess, $_.ProcessName
    }) -join "; ")
}

function Read-ReceiverLogTail {
    param([string]$Dir)
    $parts = @()
    foreach ($name in @("receiver-stdout.txt", "receiver-stderr.txt")) {
        $path = Join-Path $Dir $name
        if (Test-Path -LiteralPath $path) {
            $tail = @(
                Get-Content -LiteralPath $path -Tail 12 -ErrorAction SilentlyContinue |
                    Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
            )
            if ($tail.Count -gt 0) {
                $parts += ("{0}: {1}" -f $name, ($tail -join " | "))
            }
        }
    }
    if ($parts.Count -eq 0) {
        return "receiver logs are empty"
    }
    return ($parts -join " ; ")
}

function Invoke-AdbCapture {
    param(
        [string]$OutputPath,
        [string[]]$Arguments
    )
    $output = if ($Serial) {
        @(& $Adb -s $Serial @Arguments 2>&1)
    } else {
        @(& $Adb @Arguments 2>&1)
    }
    $output | Set-Content -Path $OutputPath -Encoding UTF8
    if ($LASTEXITCODE -ne 0) {
        throw "adb failed: $($Arguments -join ' ')"
    }
}

function Get-BrokerActivityComponent {
    if ($BrokerActivityName.StartsWith(".")) {
        return "$BrokerPackageName/$BrokerActivityName"
    }
    if ($BrokerActivityName.Contains("/")) {
        return $BrokerActivityName
    }
    return "$BrokerPackageName/$BrokerActivityName"
}

function Restart-BrokerForCase {
    param([string]$CaseId)
    if (-not $brokerSourceRequested) {
        return
    }
    $brokerRoot = Join-Path $sessionRoot ("broker-restarts\$CaseId")
    New-Item -ItemType Directory -Force -Path $brokerRoot | Out-Null
    $clientStopLog = Join-Path $brokerRoot "force-stop-clients.txt"
    foreach ($packageName in $BrokerClientPackages) {
        if ([string]::IsNullOrWhiteSpace($packageName)) {
            continue
        }
        Add-Content -Path $clientStopLog -Value ("force-stop {0}" -f $packageName) -Encoding UTF8
        Invoke-AdbCapture -OutputPath (Join-Path $brokerRoot ("force-stop-client-{0}.txt" -f ($packageName -replace "[^A-Za-z0-9_.-]", "_"))) -Arguments @("shell", "am", "force-stop", $packageName)
    }
    Start-Sleep -Milliseconds 1200
    Invoke-AdbCapture -OutputPath (Join-Path $brokerRoot "force-stop-broker.txt") -Arguments @("shell", "am", "force-stop", $BrokerPackageName)
    Start-Sleep -Milliseconds 1200
    $component = Get-BrokerActivityComponent
    Invoke-AdbCapture -OutputPath (Join-Path $brokerRoot "start.txt") -Arguments @("shell", "am", "start", "-n", $component)
    Start-Sleep -Seconds $BrokerRestartSettleSeconds
    Invoke-AdbCapture -OutputPath (Join-Path $brokerRoot "activity.txt") -Arguments @("shell", "dumpsys", "activity", "activities")
    Invoke-AdbCapture -OutputPath (Join-Path $brokerRoot "window.txt") -Arguments @("shell", "dumpsys", "window", "windows")
}

function Start-MediaProjectionReceiver {
    param([string]$Dir)
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    $preExistingListeners = @(Get-LocalTcpListeners -Port $MediaProjectionPort)
    if ($preExistingListeners.Count -gt 0) {
        throw "MediaProjection host port $MediaProjectionPort is already listening before receiver start: $(Format-LocalTcpListeners -Listeners $preExistingListeners). Choose -MediaProjectionPort for this run."
    }
    Remove-MediaProjectionReverse
    $null = Invoke-Adb -Arguments @("reverse", "tcp:$MediaProjectionPort", "tcp:$MediaProjectionPort")
    $stdout = Join-Path $Dir "receiver-stdout.txt"
    $stderr = Join-Path $Dir "receiver-stderr.txt"
    $process = Start-Process `
        -FilePath "python" `
        -ArgumentList @($receiver, "--port", $MediaProjectionPort.ToString(), "--output", $Dir, "--once", "--max-frames", $MediaProjectionMaxFrames.ToString(), "--prune-previous-payloads") `
        -PassThru `
        -WindowStyle Hidden `
        -RedirectStandardOutput $stdout `
        -RedirectStandardError $stderr
    $deadline = (Get-Date).AddSeconds(8)
    $listening = $false
    do {
        Start-Sleep -Milliseconds 150
        if ($process.HasExited) {
            throw "MediaProjection receiver exited before listening on $MediaProjectionPort. $(Read-ReceiverLogTail -Dir $Dir)"
        }
        $listeners = @(Get-LocalTcpListeners -Port $MediaProjectionPort)
        $listening = @($listeners | Where-Object { $_.OwningProcess -eq $process.Id }).Count -gt 0
    } while ((-not $listening) -and ((Get-Date) -lt $deadline))
    if (-not $listening) {
        throw "MediaProjection receiver did not start listening on $MediaProjectionPort. Current listeners: $(Format-LocalTcpListeners -Listeners $listeners). $(Read-ReceiverLogTail -Dir $Dir)"
    }
    return $process
}

function Stop-MediaProjectionReceiver {
    param([System.Diagnostics.Process]$Process)
    if ($null -eq $Process) {
        return
    }
    if (-not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
    }
}

function Remove-MediaProjectionReverse {
    try {
        if ($Serial) {
            & $Adb -s $Serial reverse --remove "tcp:$MediaProjectionPort" 2>&1 | Out-Null
        } else {
            & $Adb reverse --remove "tcp:$MediaProjectionPort" 2>&1 | Out-Null
        }
    } catch {
    }
}

function Complete-MediaProjectionCapture {
    param(
        [System.Diagnostics.Process]$Process,
        [string]$Dir,
        [string]$OutputPng
    )
    $frames = Join-Path $Dir "frames.jsonl"
    $deadline = (Get-Date).AddSeconds(45)
    while ((-not (Test-Path -LiteralPath $frames)) -and (-not $Process.HasExited) -and ((Get-Date) -lt $deadline)) {
        Start-Sleep -Milliseconds 250
    }
    if (-not (Test-Path -LiteralPath $frames)) {
        throw "MediaProjection receiver did not receive a frame for $OutputPng. Current listeners: $(Format-LocalTcpListeners -Listeners @(Get-LocalTcpListeners -Port $MediaProjectionPort)). $(Read-ReceiverLogTail -Dir $Dir). PROJECT_MEDIA app-op pregrant did not produce a capture frame; if a Meta selector is visible in-headset, grant MediaProjection manually and rerun."
    }
    if ($MediaProjectionDrainMs -gt 0 -and (-not $Process.HasExited)) {
        Start-Sleep -Milliseconds $MediaProjectionDrainMs
    }
    if (-not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
        $null = $Process.WaitForExit(5000)
    } elseif (-not $Process.WaitForExit(5000)) {
        throw "MediaProjection receiver did not exit cleanly for $OutputPng"
    }
    if (-not (Test-Path -LiteralPath $frames)) {
        throw "MediaProjection receiver wrote no frames ledger for $OutputPng"
    }
    $frameCount = @(
        Get-Content -LiteralPath $frames |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    ).Count
    if ($frameCount -lt 1) {
        throw "MediaProjection receiver wrote an empty frames ledger for $OutputPng"
    }
    Write-Host "MediaProjection frames captured for ${OutputPng}: $frameCount; converting latest frame."
    & python $converter --frames $frames --output $OutputPng --latest | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -ne 0) {
        throw "MediaProjection PNG conversion failed for $OutputPng"
    }
}

function Wait-MakepadForegroundForHzdb {
    param(
        [string]$CaseId,
        [string]$CaseRoot
    )
    $foregroundRoot = Join-Path $CaseRoot "pre-hzdb-foreground"
    New-Item -ItemType Directory -Force -Path $foregroundRoot | Out-Null
    $deadline = (Get-Date).AddSeconds(30)
    $attempt = 0
    $settled = $false
    $sawXrFrameReady = $false
    $sawProjectionReady = $false
    do {
        $attempt++
        $activityPath = Join-Path $foregroundRoot ("activity-{0:D2}.txt" -f $attempt)
        $windowPath = Join-Path $foregroundRoot ("window-{0:D2}.txt" -f $attempt)
        $logcatPath = Join-Path $foregroundRoot ("logcat-{0:D2}.txt" -f $attempt)
        Invoke-AdbCapture -OutputPath $activityPath -Arguments @("shell", "dumpsys", "activity", "activities")
        Invoke-AdbCapture -OutputPath $windowPath -Arguments @("shell", "dumpsys", "window", "windows")
        Invoke-AdbCapture -OutputPath $logcatPath -Arguments @("logcat", "-d", "-v", "threadtime")
        $activityText = Get-Content -Raw -Path $activityPath
        $windowText = Get-Content -Raw -Path $windowPath
        $logcatText = Get-Content -Raw -Path $logcatPath
        $packagePattern = [regex]::Escape($MakepadPackageName)
        $activityReady = (
            ($activityText -match "topResumedActivity=.*$packagePattern") -or
            ($activityText -match "ResumedActivity:.*$packagePattern") -or
            ($activityText -match "Resumed:.*$packagePattern")
        )
        $windowReady = (
            ($windowText -match "mCurrentFocus=.*$packagePattern") -or
            ($windowText -match "mFocusedApp=.*$packagePattern")
        )
        $appError = (
            ($activityText -match "Application Error:.*$packagePattern") -or
            ($windowText -match "Application Error:.*$packagePattern") -or
            (($logcatText -match "FATAL EXCEPTION") -and ($logcatText -match "Process:\s*$packagePattern"))
        )
        if ($appError) {
            throw "$CaseId Makepad app error was visible before HzDB capture; see $foregroundRoot"
        }
        $nonzeroXrCadenceReady = $logcatText -match "RUSTY_XR_MAKEPAD_CADENCE.*xrUpdateRateHz=(?!0\.00)"
        $xrFrameReady = ($logcatText -match "RUSTY_XR_MAKEPAD_OPENXR_END_FRAME") -or $nonzeroXrCadenceReady
        $sawXrFrameReady = $sawXrFrameReady -or $xrFrameReady
        $projectionReady = (
            ($logcatText -match "visibleCameraProjectionReady=true") -or
            $nonzeroXrCadenceReady
        )
        $sawProjectionReady = $sawProjectionReady -or $projectionReady
        if (($activityReady -or $windowReady) -and $sawXrFrameReady -and $sawProjectionReady) {
            if ((-not $settled) -and $MakepadPostRunSettleSeconds -gt 0) {
                Start-Sleep -Seconds $MakepadPostRunSettleSeconds
                $settled = $true
                continue
            }
            return
        }
        Start-Sleep -Seconds 3
    } while ((Get-Date) -lt $deadline)
    throw "$CaseId Makepad app was not foreground before HzDB capture; see $foregroundRoot"
}

function Assert-MakepadNoApplicationError {
    param(
        [string]$CaseId,
        [string]$CaseRoot
    )
    $stateRoot = Join-Path $CaseRoot "post-headset-capture-state"
    New-Item -ItemType Directory -Force -Path $stateRoot | Out-Null
    $activityPath = Join-Path $stateRoot "activity.txt"
    $windowPath = Join-Path $stateRoot "window.txt"
    $logcatPath = Join-Path $stateRoot "logcat.txt"
    Invoke-AdbCapture -OutputPath $activityPath -Arguments @("shell", "dumpsys", "activity", "activities")
    Invoke-AdbCapture -OutputPath $windowPath -Arguments @("shell", "dumpsys", "window", "windows")
    Invoke-AdbCapture -OutputPath $logcatPath -Arguments @("logcat", "-d", "-v", "threadtime")
    $activityText = Get-Content -Raw -Path $activityPath
    $windowText = Get-Content -Raw -Path $windowPath
    $logcatText = Get-Content -Raw -Path $logcatPath
    $packagePattern = [regex]::Escape($MakepadPackageName)
    $appError = (
        ($activityText -match "Application Error:.*$packagePattern") -or
        ($windowText -match "Application Error:.*$packagePattern") -or
        (($logcatText -match "FATAL EXCEPTION") -and ($logcatText -match "Process:\s*$packagePattern"))
    )
    if ($appError) {
        throw "$CaseId Makepad app error was visible after headset capture; see $stateRoot"
    }
}

function Parse-Rect {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $null
    }
    $parts = $Text.Split(",") | ForEach-Object {
        [double]::Parse($_, [System.Globalization.CultureInfo]::InvariantCulture)
    }
    if (@($parts).Count -ne 4) {
        return $null
    }
    return @($parts)
}

function Test-FullscreenRect {
    param([double[]]$Rect)
    if ($null -eq $Rect -or @($Rect).Count -ne 4) {
        return $false
    }
    return ([Math]::Abs($Rect[0]) -lt 0.0025 -and
        [Math]::Abs($Rect[1]) -lt 0.0025 -and
        [Math]::Abs($Rect[2] - 1.0) -lt 0.0025 -and
        [Math]::Abs($Rect[3] - 1.0) -lt 0.0025)
}

function Find-LatestMarkerField {
    param(
        [string]$Root,
        [string]$Field
    )
    if (-not (Test-Path -LiteralPath $Root)) {
        return $null
    }
    $pattern = [regex]::Escape($Field) + "=([^\s]+)"
    $files = Get-ChildItem -LiteralPath $Root -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Extension -in @(".txt", ".log", ".json", ".jsonl") }
    foreach ($file in ($files | Sort-Object LastWriteTime -Descending)) {
        $latestValue = $null
        try {
            foreach ($line in [System.IO.File]::ReadLines((ConvertTo-WindowsLongPath $file.FullName))) {
                $fieldMatches = [regex]::Matches($line, $pattern)
                if ($fieldMatches.Count -gt 0) {
                    $latestValue = $fieldMatches[$fieldMatches.Count - 1].Groups[1].Value.TrimEnd(",", ";")
                }
            }
        } catch {
            Write-Warning "Could not scan marker file $($file.FullName): $($_.Exception.Message)"
        }
        if ($latestValue) {
            return $latestValue
        }
    }
    return $null
}

function Find-LatestJsonScalarField {
    param(
        [string]$Root,
        [string]$Field
    )
    if (-not (Test-Path -LiteralPath $Root)) {
        return $null
    }
    $pattern = '"' + [regex]::Escape($Field) + '"\s*:\s*([^,\r\n}]+)'
    $files = Get-ChildItem -LiteralPath $Root -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Extension -in @(".json", ".jsonl") }
    foreach ($file in ($files | Sort-Object LastWriteTime -Descending)) {
        $latestValue = $null
        try {
            foreach ($line in [System.IO.File]::ReadLines((ConvertTo-WindowsLongPath $file.FullName))) {
                $fieldMatches = [regex]::Matches($line, $pattern)
                if ($fieldMatches.Count -gt 0) {
                    $latestValue = $fieldMatches[$fieldMatches.Count - 1].Groups[1].Value.Trim().Trim('"')
                }
            }
        } catch {
            Write-Warning "Could not scan JSON marker file $($file.FullName): $($_.Exception.Message)"
        }
        if ($latestValue) {
            return $latestValue
        }
    }
    return $null
}

function Wait-LatestMarkerField {
    param(
        [string]$Root,
        [string]$Field,
        [int]$TimeoutSeconds = 8
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        $value = Find-LatestMarkerField -Root $Root -Field $Field
        if (-not [string]::IsNullOrWhiteSpace($value)) {
            return $value
        }
        Start-Sleep -Milliseconds 500
    } while ((Get-Date) -lt $deadline)
    return $null
}

function ConvertTo-CanonicalSourceEyeMapping {
    param([string]$Value)
    if ([string]::IsNullOrWhiteSpace($Value)) {
        return $null
    }
    $normalized = $Value.Trim().TrimEnd(",", ";").ToLowerInvariant()
    if ($normalized -in @("left-right", "display-left-from-left", "display-left-from-left-source")) {
        return "display-left-from-left-source"
    }
    if ($normalized -in @("right-left", "display-left-from-right", "display-left-from-right-source")) {
        return "display-left-from-right-source"
    }
    return $normalized
}

function Assert-MakepadSourceEyeMapping {
    param(
        [string]$CaseId,
        [string]$CaseRoot
    )
    $mapping = Wait-LatestMarkerField -Root $CaseRoot -Field "sourceEyeMapping" -TimeoutSeconds 12
    $canonicalMapping = ConvertTo-CanonicalSourceEyeMapping -Value $mapping
    $evidence = [ordered]@{
        caseId = $CaseId
        artifactDir = $CaseRoot
        expectedSourceEyeMapping = $ExpectedMakepadSourceEyeMapping
        observedSourceEyeMapping = $mapping
        observedCanonicalSourceEyeMapping = $canonicalMapping
    }
    $evidencePath = Join-Path $sessionRoot ("makepad-source-eye-mapping-evidence-$CaseId.json")
    $evidence | ConvertTo-Json -Depth 4 | Set-Content -Path $evidencePath -Encoding UTF8

    if ([string]::IsNullOrWhiteSpace($mapping)) {
        throw "[$CaseId] Makepad sourceEyeMapping marker was not found; rejecting parity evidence. See $evidencePath"
    }
    if ($canonicalMapping -ne $ExpectedMakepadSourceEyeMapping) {
        throw "[$CaseId] Makepad sourceEyeMapping expected $ExpectedMakepadSourceEyeMapping but saw '$mapping' (canonical '$canonicalMapping'); rebuild without the inverted-source diagnostic override. See $evidencePath"
    }
}

function Assert-BoundedFootprintEvidence {
    param(
        [string]$CaseId,
        [string]$ArtifactDir
    )
    $leftProjectionArea = Parse-Rect (Wait-LatestMarkerField -Root $ArtifactDir -Field "leftProjectionAreaScreenUvRect")
    $rightProjectionArea = Parse-Rect (Wait-LatestMarkerField -Root $ArtifactDir -Field "rightProjectionAreaScreenUvRect")
    $leftExpected = Parse-Rect (Wait-LatestMarkerField -Root $ArtifactDir -Field "leftExpectedSourceValidScreenUvRect")
    $rightExpected = Parse-Rect (Wait-LatestMarkerField -Root $ArtifactDir -Field "rightExpectedSourceValidScreenUvRect")
    $projectionAreaFullscreen = (Test-FullscreenRect $leftProjectionArea) -or (Test-FullscreenRect $rightProjectionArea)
    $sourceValidFootprintsPresent = ($null -ne $leftExpected) -and ($null -ne $rightExpected)
    $sourceValidFootprintFullscreen = $sourceValidFootprintsPresent -and ((Test-FullscreenRect $leftExpected) -or (Test-FullscreenRect $rightExpected))
    $sourceValidFootprintBounded = $sourceValidFootprintsPresent -and -not $sourceValidFootprintFullscreen
    $sourceValidCoverage = if (-not $sourceValidFootprintsPresent) {
        "missing"
    } elseif ($sourceValidFootprintFullscreen) {
        "fullscreen"
    } else {
        "bounded"
    }
    $renderSurfaceCoverage = if ($projectionAreaFullscreen) {
        "fullscreen"
    } else {
        "bounded"
    }
    $footprintContract = if (-not $sourceValidFootprintsPresent) {
        "missing-source-valid-footprint"
    } elseif ($sourceValidFootprintFullscreen) {
        "fullscreen-source-valid-footprint"
    } elseif ($projectionAreaFullscreen) {
        "fullscreen-render-surface-bounded-source-valid-footprint"
    } else {
        "bounded-render-surface-bounded-source-valid-footprint"
    }
    $contractSignals = @()
    if ($projectionAreaFullscreen -and $sourceValidFootprintBounded) {
        $contractSignals += "fullscreen-render-surface-with-bounded-source-valid-footprint"
    }

    $evidence = [ordered]@{
        caseId = $CaseId
        artifactDir = $ArtifactDir
        leftProjectionAreaScreenUvRect = $leftProjectionArea
        rightProjectionAreaScreenUvRect = $rightProjectionArea
        leftExpectedSourceValidScreenUvRect = $leftExpected
        rightExpectedSourceValidScreenUvRect = $rightExpected
        projectionAreaFullscreen = $projectionAreaFullscreen
        effectiveFootprintFullscreen = $sourceValidFootprintFullscreen
        renderSurfaceFullscreen = $projectionAreaFullscreen
        renderSurfaceUvCoverage = $renderSurfaceCoverage
        sourceValidFootprintFullscreen = $sourceValidFootprintFullscreen
        sourceValidFootprintBounded = $sourceValidFootprintBounded
        sourceValidFootprintUvCoverage = $sourceValidCoverage
        footprintContract = $footprintContract
        contractSignals = $contractSignals
        status = "ok"
        issues = @()
    }
    $issues = @()
    $evidencePath = Join-Path $sessionRoot ("bounded-footprint-evidence-$CaseId.json")

    if ($null -eq $leftExpected -or $null -eq $rightExpected) {
        $issues += "missing-expected-source-valid-footprint"
    }
    elseif ($sourceValidFootprintFullscreen) {
        $issues += "effective-source-valid-footprint-fullscreen"
    }

    $evidence["status"] = if ($issues.Count -eq 0) { "ok" } elseif ($issues -contains "effective-source-valid-footprint-fullscreen") { "invalid" } else { "warning" }
    $evidence["issues"] = $issues
    $evidence | ConvertTo-Json -Depth 5 | Set-Content -Path $evidencePath -Encoding UTF8
    $script:boundedFootprintEvidenceRecords.Add($evidence)

    if ($issues.Count -gt 0) {
        $message = "[$CaseId] bounded footprint evidence status=$($evidence["status"]) issues=$($issues -join ','); see $evidencePath"
        if ($FailOnAnalyzerIssue -and $evidence["status"] -eq "invalid") {
            throw $message
        }
        Write-Warning $message
    }
}

function Copy-HeadsetCaptureFromProfileRun {
    param(
        [string]$ProfileRoot,
        [string]$RuntimeProfile,
        [string]$OutputPng
    )
    $capturePattern = if ($HeadsetCaptureProvider -eq "hzdb") { "*-hzdb-screencap.png" } else { "*-screencap.png" }
    $captureLabel = if ($HeadsetCaptureProvider -eq "hzdb") { "HzDB screenshot" } else { "fast ADB screenshot" }
    $lastError = $null
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        try {
            $latest = Get-ChildItem -LiteralPath (ConvertTo-WindowsLongPath $ProfileRoot) -Directory -ErrorAction Stop |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if (-not $latest) {
                $lastError = "No profile run directory found under $ProfileRoot"
            } else {
                $sourceCandidates = Get-ChildItem -LiteralPath (ConvertTo-WindowsLongPath $latest.FullName) -Filter $capturePattern -ErrorAction Stop
                if ($HeadsetCaptureProvider -eq "fast-adb") {
                    $sourceCandidates = @($sourceCandidates | Where-Object { $_.Name -notlike "*-hzdb-screencap.png" })
                }
                $source = $sourceCandidates |
                    Sort-Object LastWriteTime -Descending |
                    Select-Object -First 1
                if ($source) {
                    [System.IO.File]::Copy(
                        (ConvertTo-WindowsLongPath $source.FullName),
                        (ConvertTo-WindowsLongPath $OutputPng),
                        $true)
                    return $latest.FullName
                }
                $lastError = "$captureLabel not found under $($latest.FullName)"
            }
        } catch {
            $lastError = $_.Exception.Message
        }
        Start-Sleep -Milliseconds 500
    }
    throw $lastError
}

function Copy-HeadsetCaptureFromMakepadRun {
    param(
        [string]$CaseRoot,
        [string]$OutputPng
    )
    $sourceCandidates = @(Get-ChildItem -LiteralPath (ConvertTo-WindowsLongPath $CaseRoot) -Recurse -Filter "*.png" -ErrorAction Stop |
        Where-Object { $_.FullName -match "\\screenshots\\" -and $_.Name -match "frame-00\.png$" } |
        Sort-Object LastWriteTime -Descending)
    $source = $sourceCandidates |
        Where-Object { $_.FullName -match "\\[^\\]+-final\\screenshots\\" } |
        Select-Object -First 1
    if (-not $source) {
        throw "Makepad final fast ADB screenshot not found under $CaseRoot"
    }
    [System.IO.File]::Copy(
        (ConvertTo-WindowsLongPath $source.FullName),
        (ConvertTo-WindowsLongPath $OutputPng),
        $true)
    return $source.FullName
}

function Invoke-HwbOrGlesCase {
    param(
        [string]$Lane,
        [string]$Mode,
        [string]$Catalog,
        [string]$AppId,
        [string]$DeviceProfile,
        [string]$RuntimeProfile,
        [string]$Apk,
        [string]$Override
    )
    $caseId = "$Lane-$Mode"
    if ($HeadsetCaptureProvider -eq "fast-adb") {
        Write-Host "[$caseId] fast ADB headset capture starting"
    } else {
        Write-Host "[$caseId] HzDB headset capture starting"
    }
    $caseRoot = Join-Path $profileRunRoot $caseId
    $mediaRoot = Join-Path $sessionRoot "mediaprojection\$caseId"
    $mediaPng = Join-Path $screenshotsRoot "$caseId-mediaprojection.png"
    $headsetPng = Join-Path $screenshotsRoot "$caseId-headset.png"
    $receiverProcess = $null

    $profileArgs = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", $profileRunner,
        "-Serial", $Serial,
        "-Adb", $Adb,
        "-Npx", $Npx,
        "-Catalog", $Catalog,
        "-AppId", $AppId,
        "-DeviceProfile", $DeviceProfile,
        "-RuntimeProfile", $RuntimeProfile,
        "-RunRoot", $caseRoot,
        "-WarmupSeconds", $WarmupSeconds.ToString(),
        "-FreshnessFrames", "1",
        "-SkipProximityHold",
        "-LogcatLines", "16000",
        "-ProjectionPropertyHygiene", "clear",
        "-ProjectionRuntimeReadback", $effectiveProjectionRuntimeReadback,
        "-Override", $Override
    )
    if ($HeadsetCaptureProvider -eq "hzdb") {
        $profileArgs += "-CaptureHzdbScreencap"
    }
    if ($Install) {
        $profileArgs += @("-Install", "-Apk", (Resolve-RepoPath $Apk))
    }
    try {
        Invoke-TimedStep -CaseId $caseId -Step "broker-restart" -Action { Restart-BrokerForCase -CaseId $caseId }
        if (-not $SkipMediaProjection) {
            $receiverProcess = Invoke-TimedStep -CaseId $caseId -Step "mediaprojection-receiver-start" -Action { Start-MediaProjectionReceiver -Dir $mediaRoot }
        }
        Invoke-TimedStep -CaseId $caseId -Step "launch-settle-adb-capture" -Action {
            & powershell @profileArgs | ForEach-Object { Write-Host $_ }
            if ($LASTEXITCODE -ne 0) {
                throw "$caseId profile run failed"
            }
        }
        if (-not $SkipMediaProjection) {
            Invoke-TimedStep -CaseId $caseId -Step "mediaprojection-complete" -Action { Complete-MediaProjectionCapture -Process $receiverProcess -Dir $mediaRoot -OutputPng $mediaPng }
        }
        $artifactDir = Invoke-TimedStep -CaseId $caseId -Step "headset-capture-copy" -Action { Copy-HeadsetCaptureFromProfileRun -ProfileRoot $caseRoot -RuntimeProfile $RuntimeProfile -OutputPng $headsetPng }
        Invoke-TimedStep -CaseId $caseId -Step "bounded-footprint-evidence" -Action { Assert-BoundedFootprintEvidence -CaseId $caseId -ArtifactDir $artifactDir }
    }
    finally {
        if (-not $SkipMediaProjection) {
            Invoke-TimedStep -CaseId $caseId -Step "mediaprojection-cleanup" -Action {
                Stop-MediaProjectionReceiver -Process $receiverProcess
                Remove-MediaProjectionReverse
            }
        }
    }
    Write-Host "[$caseId] captured headset provider=$HeadsetCaptureProvider"
    return [ordered]@{
        id = $caseId
        lane = $Lane
        mode = $Mode
        runtimeProfile = $RuntimeProfile
        artifactDir = $artifactDir
        mediaProjection = if ($SkipMediaProjection) { $null } else { $mediaPng }
        hzdb = $headsetPng
        headsetCapture = $headsetPng
        headsetCaptureProvider = $HeadsetCaptureProvider
        brokerH264SourceMode = $SourceMode
        brokerH264SyntheticPattern = if ($SourceMode -eq "broker-synthetic") { $BrokerH264SyntheticPattern } else { $null }
        brokerH264SyntheticProjectionProfile = if ($SourceMode -eq "broker-synthetic") { $BrokerH264SyntheticProjectionProfile } else { $null }
        processingLayer = $ProcessingLayer
        blurRadiusPx = $BlurRadiusPx
    }
}

function Invoke-MakepadCase {
    param(
        [string]$Mode,
        [string]$CameraProjectionMode,
        [string]$ProjectionGeometryProfile,
        [string]$CaseSourceMode = "direct-camera"
    )
    $caseId = "makepad-$Mode"
    if ($HeadsetCaptureProvider -eq "fast-adb") {
        Write-Host "[$caseId] fast ADB headset capture starting"
    } else {
        Write-Host "[$caseId] HzDB headset capture starting"
    }
    $caseRoot = Join-Path $makepadRunRoot $caseId
    $mediaRoot = Join-Path $sessionRoot "mediaprojection\$caseId"
    $mediaPng = Join-Path $screenshotsRoot "$caseId-mediaprojection.png"
    $headsetPng = Join-Path $screenshotsRoot "$caseId-headset.png"
    $receiverProcess = $null

    $adbPath = if (Test-Path -LiteralPath $Adb) {
        (Resolve-Path -LiteralPath $Adb).Path
    } else {
        $null
    }
    if ($adbPath) {
        $adbDir = Split-Path -Parent $adbPath
        $env:PATH = "$adbDir;$env:PATH"
    }
    $makepadSampleSecondsForRun = [Math]::Max($MakepadSampleSeconds, $WarmupSeconds)
    $makepadArgs = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", $makepadRunner,
        "-Serial", $Serial,
        "-Apk", (Resolve-RepoPath $MakepadApk),
        "-PackageName", $MakepadPackageName,
        "-OutDir", $caseRoot,
        "-StartupTimeoutSeconds", $MakepadStartupTimeoutSeconds.ToString(),
        "-SampleSeconds", $makepadSampleSecondsForRun.ToString(),
        "-FreshnessFrames", "1",
        "-FreshnessIntervalSeconds", "1",
        "-PreferDirectVrActivity",
        "-CameraProjectionMode", $CameraProjectionMode,
        "-CameraProjectionGeometryProfile", $ProjectionGeometryProfile,
        "-ProjectionDepthMeters", "1.434085",
        "-CameraPreviewFovYDegrees", "69.763084",
        "-CameraPreviewOffsetYMeters", "-0.168832",
        "-CameraRawOverlayOverscan", "1.0",
        "-ProjectionAreaOffsetXUv", "0.0",
        "-ProjectionAreaOffsetYUv", "0.0",
        "-ProjectionAreaScaleX", "1.0",
        "-ProjectionAreaScaleY", "1.0",
        "-ProjectionAreaRadiusXUv", $projectionAreaRadiusXUv,
        "-ProjectionAreaRadiusYUv", $projectionAreaRadiusYUv,
        "-ProjectionAreaCornerRadiusUv", $projectionAreaCornerRadiusUv,
        "-ProjectionAreaOpacity", (Format-LaunchFloat -Value $ProjectionAreaOpacity),
        "-ProjectionBorderOpacity", (Format-LaunchFloat -Value $ProjectionBorderOpacity),
        "-ProjectionBorderPolicy", $ProjectionBorderPolicy,
        "-ProcessingLayer", $ProcessingLayer,
        "-ProjectionPropertyHygiene", "clear",
        "-ProjectionRuntimeReadback", $effectiveProjectionRuntimeReadback,
        "-BlurRadiusPx", $blurRadiusPxText
    )
    if (-not $SkipMediaProjection) {
        $makepadArgs += @(
            "-MediaProjection",
            "-MediaProjectionPort", $MediaProjectionPort.ToString()
        )
    }
    if ($UseResolvedProjectionRuntime) {
        $makepadArgs += "-UseResolvedProjectionRuntime"
    }
    if (Get-MakepadNativePassthroughRequested) {
        $makepadArgs += "-EnableNativePassthrough"
    }
    if ($CaseSourceMode -eq "broker-camera") {
        $makepadArgs += @(
            "-UseBrokerH264Camera",
            "-BrokerH264LeftCameraId", $BrokerH264LeftCameraId,
            "-BrokerH264RightCameraId", $BrokerH264RightCameraId,
            "-BrokerH264ProjectionGeometryProfile", $ProjectionGeometryProfile,
            "-BrokerH264CaptureMs", "0",
            "-BrokerH264MaxPackets", "0",
            "-BrokerH264FrameRateHz", $BrokerH264FrameRateHz.ToString(),
            "-BrokerH264BitrateBps", $BrokerH264BitrateBps.ToString(),
            "-BrokerH264LeftStreamPort", $BrokerH264LeftStreamPort.ToString(),
            "-BrokerH264RightStreamPort", $BrokerH264RightStreamPort.ToString()
        )
    }
    elseif ($CaseSourceMode -eq "broker-synthetic") {
        $makepadArgs += @(
            "-UseBrokerH264Synthetic",
            "-BrokerH264SyntheticPattern", $BrokerH264SyntheticPattern,
            "-BrokerH264SyntheticProjectionProfile", $BrokerH264SyntheticProjectionProfile,
            "-BrokerH264ProjectionGeometryProfile", $BrokerH264SyntheticProjectionProfile,
            "-BrokerH264LeftCameraId", $BrokerH264LeftCameraId,
            "-BrokerH264RightCameraId", $BrokerH264RightCameraId,
            "-BrokerH264CaptureMs", "0",
            "-BrokerH264MaxPackets", "0",
            "-BrokerH264FrameRateHz", $BrokerH264FrameRateHz.ToString(),
            "-BrokerH264BitrateBps", $BrokerH264BitrateBps.ToString(),
            "-BrokerH264LeftStreamPort", $BrokerH264LeftStreamPort.ToString(),
            "-BrokerH264RightStreamPort", $BrokerH264RightStreamPort.ToString()
        )
    }
    if (-not $Install) {
        $makepadArgs += "-SkipInstall"
    }
    try {
        Invoke-TimedStep -CaseId $caseId -Step "broker-restart" -Action { Restart-BrokerForCase -CaseId $caseId }
        if (-not $SkipMediaProjection) {
            $receiverProcess = Invoke-TimedStep -CaseId $caseId -Step "mediaprojection-receiver-start" -Action { Start-MediaProjectionReceiver -Dir $mediaRoot }
        }
        Invoke-TimedStep -CaseId $caseId -Step "makepad-gate-launch-adb-capture" -Action {
            & powershell @makepadArgs | ForEach-Object { Write-Host $_ }
            if ($LASTEXITCODE -ne 0) {
                throw "$caseId Makepad run failed"
            }
        }
        Invoke-TimedStep -CaseId $caseId -Step "makepad-source-eye-gate" -Action { Assert-MakepadSourceEyeMapping -CaseId $caseId -CaseRoot $caseRoot }

        if ($HeadsetCaptureProvider -eq "hzdb") {
            Invoke-TimedStep -CaseId $caseId -Step "makepad-foreground-before-hzdb" -Action { Wait-MakepadForegroundForHzdb -CaseId $caseId -CaseRoot $caseRoot }
            Invoke-TimedStep -CaseId $caseId -Step "hzdb-screencap" -Action {
                $hzdbArgs = @("-y", "@meta-quest/hzdb", "capture", "screenshot")
                if ($Serial) {
                    $hzdbArgs += @("--device", $Serial)
                }
                $hzdbArgs += @("--method", "screencap", "--output", $headsetPng)
                & $Npx @hzdbArgs | ForEach-Object { Write-Host $_ }
                if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $headsetPng)) {
                    throw "$caseId HzDB capture failed"
                }
            }
        } else {
            Invoke-TimedStep -CaseId $caseId -Step "headset-capture-copy" -Action { Copy-HeadsetCaptureFromMakepadRun -CaseRoot $caseRoot -OutputPng $headsetPng | Out-Null }
        }
        Invoke-TimedStep -CaseId $caseId -Step "makepad-application-error-check" -Action { Assert-MakepadNoApplicationError -CaseId $caseId -CaseRoot $caseRoot }
        if (-not $SkipMediaProjection) {
            Invoke-TimedStep -CaseId $caseId -Step "mediaprojection-complete" -Action { Complete-MediaProjectionCapture -Process $receiverProcess -Dir $mediaRoot -OutputPng $mediaPng }
        }
        Invoke-TimedStep -CaseId $caseId -Step "bounded-footprint-evidence" -Action { Assert-BoundedFootprintEvidence -CaseId $caseId -ArtifactDir $caseRoot }
    }
    finally {
        if (-not $SkipMediaProjection) {
            Invoke-TimedStep -CaseId $caseId -Step "mediaprojection-cleanup" -Action {
                Stop-MediaProjectionReceiver -Process $receiverProcess
                Remove-MediaProjectionReverse
            }
        }
    }
    Write-Host "[$caseId] captured headset provider=$HeadsetCaptureProvider"
    return [ordered]@{
        id = $caseId
        lane = "makepad"
        mode = $Mode
        cameraProjectionMode = $CameraProjectionMode
        runtimeProfile = $ProjectionGeometryProfile
        artifactDir = $caseRoot
        mediaProjection = if ($SkipMediaProjection) { $null } else { $mediaPng }
        hzdb = $headsetPng
        headsetCapture = $headsetPng
        headsetCaptureProvider = $HeadsetCaptureProvider
        brokerH264SourceMode = $SourceMode
        brokerH264SyntheticPattern = if ($SourceMode -eq "broker-synthetic") { $BrokerH264SyntheticPattern } else { $null }
        brokerH264SyntheticProjectionProfile = if ($SourceMode -eq "broker-synthetic") { $BrokerH264SyntheticProjectionProfile } else { $null }
        processingLayer = $ProcessingLayer
        blurRadiusPx = $BlurRadiusPx
    }
}

$hwbCatalog = "examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json"
$glesCatalog = "examples\quest-gl-openxr-video-stack-apk\catalog\rusty-xr-quest-gl-openxr-video-stack.catalog.json"
$BrokerClientPackages = @(
    (Get-CatalogPackageName -Catalog $hwbCatalog),
    (Get-CatalogPackageName -Catalog $glesCatalog),
    $MakepadPackageName
) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique
$hwbCanvasRuntimeProfile = if ($brokerSourceRequested) {
    "broker-h264-stereo-live-world-canvas-mediaprojection"
}
else {
    "camera-stereo-gpu-composite-world-canvas-native-aligned-mediaprojection"
}
$hwbCustomRuntimeProfile = if ($brokerSourceRequested) {
    "broker-h264-stereo-live-openxr-projection-full-feed-control"
}
else {
    "camera-stereo-gpu-composite-camera-footprint-canvas-equivalent-depth1"
}
$glesCanvasRuntimeProfile = if ($brokerSourceRequested) {
    "gles-broker-camera-h264-oes-projection"
}
elseif ($SkipMediaProjection) {
    "gles-direct-camera2-oes-projection"
}
else {
    "gles-direct-camera2-oes-world-canvas-mediaprojection"
}
$glesCustomRuntimeProfile = if ($brokerSourceRequested) {
    "gles-broker-camera-h264-oes-projection"
}
elseif ($SkipMediaProjection) {
    "gles-direct-camera2-oes-projection"
}
else {
    "gles-direct-camera2-oes-camera-projection-mediaprojection"
}
$projectionAreaOverride = if ($BoundedCanvasProjectionArea) { $boundedProjectionAreaOverride } else { "" }
$hwbCanvasSourceOverride = if ($brokerSourceRequested) { Get-BrokerH264Override -ProjectionGeometryProfile "full-frame-diagnostic" } else { "" }
$hwbCustomSourceOverride = if ($brokerSourceRequested) { Get-BrokerH264Override -ProjectionGeometryProfile "camera-projection" } else { "" }
$glesCanvasSourceOverride = if ($brokerSourceRequested) { Get-BrokerH264Override -ProjectionGeometryProfile "full-frame-diagnostic" } else { "" }
$glesCustomSourceOverride = if ($brokerSourceRequested) { Get-BrokerH264Override -ProjectionGeometryProfile "camera-projection" } else { "" }
$records = @()
if (Test-LaneEnabled -Lane "hwb") {
    $records += Invoke-HwbOrGlesCase `
        -Lane "hwb" `
        -Mode "canvas" `
        -Catalog $hwbCatalog `
        -AppId "rusty-xr-quest-composite-layer" `
        -DeviceProfile "xr-composite-comparison-level-5" `
        -RuntimeProfile $hwbCanvasRuntimeProfile `
        -Apk $HwbApk `
        -Override (Join-OverrideValues -Values @(
            "rustyxr.cameraProjectionMode=world-canvas",
            "rustyxr.cameraProjectionGeometryProfile=full-frame-diagnostic",
            (Get-HwbProjectionStyleOverride -Mode "canvas"),
            $projectionAreaOverride,
            $hwbCanvasSourceOverride,
            $surfaceOverride
        ))
    $records += Invoke-HwbOrGlesCase `
        -Lane "hwb" `
        -Mode "custom" `
        -Catalog $hwbCatalog `
        -AppId "rusty-xr-quest-composite-layer" `
        -DeviceProfile "xr-composite-comparison-level-5" `
        -RuntimeProfile $hwbCustomRuntimeProfile `
        -Apk $HwbApk `
        -Override (Join-OverrideValues -Values @(
            "rustyxr.cameraProjectionMode=display-screen-homography",
            "rustyxr.cameraProjectionGeometryProfile=camera-projection",
            (Get-HwbProjectionStyleOverride -Mode "custom"),
            $projectionAreaOverride,
            $hwbCustomSourceOverride,
            $surfaceOverride
        ))
}
if (Test-LaneEnabled -Lane "oes") {
    $records += Invoke-HwbOrGlesCase `
        -Lane "oes" `
        -Mode "canvas" `
        -Catalog $glesCatalog `
        -AppId "rusty-xr-quest-gl-openxr-video-stack" `
        -DeviceProfile "gles-openxr-comparison-level-5" `
        -RuntimeProfile $glesCanvasRuntimeProfile `
        -Apk $GlesApk `
        -Override (Join-OverrideValues -Values @(
            "rustyxr.cameraProjectionMode=world-canvas",
            "rustyxr.cameraProjectionGeometryProfile=full-frame-diagnostic",
            "rustyxr.directCamera2OesProjectionGeometryProfile=full-frame-diagnostic",
            (Get-GlesProjectionStyleOverride),
            $projectionAreaOverride,
            $glesCanvasSourceOverride,
            $surfaceOverride
        ))
    $records += Invoke-HwbOrGlesCase `
        -Lane "oes" `
        -Mode "custom" `
        -Catalog $glesCatalog `
        -AppId "rusty-xr-quest-gl-openxr-video-stack" `
        -DeviceProfile "gles-openxr-comparison-level-5" `
        -RuntimeProfile $glesCustomRuntimeProfile `
        -Apk $GlesApk `
        -Override (Join-OverrideValues -Values @(
            "rustyxr.cameraProjectionMode=display-screen-homography",
            "rustyxr.cameraProjectionGeometryProfile=camera-projection",
            "rustyxr.directCamera2OesProjectionGeometryProfile=camera-projection",
            (Get-GlesProjectionStyleOverride),
            $projectionAreaOverride,
            $glesCustomSourceOverride,
            $surfaceOverride
        ))
}
if (Test-LaneEnabled -Lane "makepad") {
    $records += Invoke-MakepadCase -Mode "canvas" -CameraProjectionMode "world-canvas" -ProjectionGeometryProfile "full-frame-diagnostic" -CaseSourceMode $SourceMode
    $records += Invoke-MakepadCase -Mode "custom" -CameraProjectionMode "display-screen-homography" -ProjectionGeometryProfile "camera-projection" -CaseSourceMode $SourceMode
}

$contactSheetPath = Join-Path $sessionRoot "canvas-custom-projection-parity-results.png"
$screenSpaceAnalysisDir = Join-Path $sessionRoot "screen-space-analysis"
$headsetCaptureLabel = if ($HeadsetCaptureProvider -eq "fast-adb") { "fast ADB screencap" } else { "HzDB screencap" }

$summary = [ordered]@{
    schemaVersion = "rusty.xr.canvas-custom-projection-parity-suite.v1"
    capturedAt = (Get-Date).ToString("o")
    serial = $Serial
    sourceMode = $SourceMode
    evidenceMode = $EvidenceMode
    sessionRoot = $sessionRoot
    screenshotsRoot = $screenshotsRoot
    contactSheet = $contactSheetPath
    screenSpaceAnalysis = $screenSpaceAnalysisDir
    timingJsonl = $timingPath
    timingSummary = $timingSummaryPath
    headsetCaptureProvider = $HeadsetCaptureProvider
    captureContract = [ordered]@{
        evidenceMode = $EvidenceMode
        mediaProjectionEnabled = -not [bool]$SkipMediaProjection
        analyzerEnabled = -not [bool]$SkipAnalyzer
        contactSheetEnabled = $true
        timingEnabled = $true
        projectionPropertyHygiene = "clear"
        projectionRuntimeReadback = $effectiveProjectionRuntimeReadback
        geometryWitness = $headsetCaptureLabel
        modeSemantics = switch ($EvidenceMode) {
            "fast-visual" { "Fast visual capture mode: fast ADB headset screenshots. MediaProjection, analyzer, and projection border policy are controlled by their own switches." }
            "full-evidence" { "Full diagnostic capture mode: HzDB headset screenshots, MediaProjection receiver, analyzer overlays/contracts, contact sheet, and timing summary. Projection border policy is controlled separately." }
            default { "Custom mode: explicit capture/analyzer/projection switches define the run contract." }
        }
    }
    geometry = [ordered]@{
        projectionDepthMeters = 1.434085
        cameraPreviewFovYDegrees = 69.763084
        cameraPreviewOffsetYMeters = -0.168832
        cameraRawOverlayOverscan = 1.0
        projectionBorderPolicy = $ProjectionBorderPolicy
        processingLayer = $ProcessingLayer
        blurRadiusPx = $BlurRadiusPx
        projectionAreaOpacity = $ProjectionAreaOpacity
        projectionBorderOpacity = $ProjectionBorderOpacity
        boundedCanvasProjectionArea = [bool]$BoundedCanvasProjectionArea
        skipMediaProjection = [bool]$SkipMediaProjection
        useResolvedProjectionRuntime = [bool]$UseResolvedProjectionRuntime
        projectionRuntimeReadback = $effectiveProjectionRuntimeReadback
        projectionAreaRadiusXUv = [double]$projectionAreaRadiusXUv
        projectionAreaRadiusYUv = [double]$projectionAreaRadiusYUv
        projectionAreaCornerRadiusUv = [double]$projectionAreaCornerRadiusUv
        makepadStartupTimeoutSeconds = $MakepadStartupTimeoutSeconds
        makepadSampleSeconds = [Math]::Max($MakepadSampleSeconds, $WarmupSeconds)
        makepadPostRunSettleSeconds = $MakepadPostRunSettleSeconds
        expectedMakepadSourceEyeMapping = $ExpectedMakepadSourceEyeMapping
        failOnAnalyzerIssue = [bool]$FailOnAnalyzerIssue
        skipAnalyzer = [bool]$SkipAnalyzer
    }
    brokerH264 = [ordered]@{
        sourceMode = $SourceMode
        width = 1280
        height = 1280
        aspectRatio = 1.0
        leftCameraId = $BrokerH264LeftCameraId
        rightCameraId = $BrokerH264RightCameraId
        leftStreamPort = $BrokerH264LeftStreamPort
        rightStreamPort = $BrokerH264RightStreamPort
        frameRateHz = $BrokerH264FrameRateHz
        bitrateBps = $BrokerH264BitrateBps
        syntheticPattern = if ($SourceMode -eq "broker-synthetic") { $BrokerH264SyntheticPattern } else { $null }
        syntheticProjectionProfile = if ($SourceMode -eq "broker-synthetic") { $BrokerH264SyntheticProjectionProfile } else { $null }
        syntheticNote = if ($SourceMode -eq "broker-synthetic") { "Synthetic diagnostic stimulus uses the requested broker synthetic projection metadata profile and the same broker H.264 decode/render stack as broker-camera; with camera-matched metadata, source pixels are diagnostic blur data while camera-derived projection metadata stays comparable." } else { $null }
    }
    captureRouteNotes = @(
        $(if ($SkipMediaProjection) { "MediaProjection capture is disabled for this run; $headsetCaptureLabel is the only image evidence." } else { "MediaProjection captures are latest-frame app/display-capture evidence for the rendered camera window after the profile run." }),
        "Headset captures use provider '$HeadsetCaptureProvider' ($headsetCaptureLabel); this is the geometry authority for per-eye footprint diagnostics, and the contact sheet overlays analyzer boxes on that column.",
        $(if ($SkipMediaProjection) { "Projection parity should be judged from headset capture only." } else { "MediaProjection is display/app-window mirror evidence and may visually align with a different headset eye by renderer; do not use its apparent eye index as source-eye parity proof." }),
        "Broker-source runs restart the broker service before each condition. Broker-camera requests physical Camera2 H.264 streams; broker-synthetic requests the diagnostic H.264 stimulus with camera-matched metadata for synthetic blur evidence."
    )
    boundedFootprintEvidence = @($script:boundedFootprintEvidenceRecords)
    records = $records
}
$summaryPath = Join-Path $sessionRoot "canvas-custom-projection-parity-suite-summary.json"
$summary | ConvertTo-Json -Depth 7 | Set-Content -Path $summaryPath -Encoding UTF8

$analysisStatus = [ordered]@{
    skipped = [bool]$SkipAnalyzer
    status = if ($SkipAnalyzer) { "skipped" } else { "pending" }
    outDir = $screenSpaceAnalysisDir
    error = ""
}
$contactSheetStatus = [ordered]@{
    skipped = $false
    status = "pending"
    path = $contactSheetPath
    error = ""
}
$artifactValidationStatus = [ordered]@{
    skipped = $false
    status = "pending"
    validator = $artifactValidator
    error = ""
}

if (-not $SkipAnalyzer) {
    try {
        $analysisArgs = @($sessionRoot, "--out-dir", $screenSpaceAnalysisDir)
        if ($ProjectionBorderPolicy -ne "solid-red") {
            $analysisArgs += "--allow-visible-fallback"
        }
        Invoke-TimedStep -CaseId "suite" -Step "screen-space-analysis" -Action {
            & python $screenSpaceAnalyzer @analysisArgs | ForEach-Object { Write-Host $_ }
            if ($LASTEXITCODE -ne 0) {
                throw "Canvas/custom screen-space analysis failed with exit code $LASTEXITCODE"
            }
        }
        $analysisStatus["status"] = "ok"
    } catch {
        $analysisStatus["status"] = "failed"
        $analysisStatus["error"] = $_.Exception.Message
        Write-Warning $analysisStatus["error"]
        if ($FailOnAnalyzerIssue) {
            throw
        }
    }
}

try {
    Invoke-TimedStep -CaseId "suite" -Step "contact-sheet" -Action {
        & python $contactSheetBuilder --session-root $sessionRoot --analysis-dir $screenSpaceAnalysisDir --output $contactSheetPath | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0) {
            throw "Canvas/custom parity contact sheet generation failed with exit code $LASTEXITCODE"
        }
    }
    $contactSheetStatus["status"] = "ok"
} catch {
    $contactSheetStatus["status"] = "failed"
    $contactSheetStatus["error"] = $_.Exception.Message
    Write-Warning $contactSheetStatus["error"]
    if ($FailOnAnalyzerIssue) {
        throw
    }
}

$timingSummary = New-TimingSummary
$timingSummary | ConvertTo-Json -Depth 8 | Set-Content -Path $timingSummaryPath -Encoding UTF8
$summary["analysis"] = $analysisStatus
$summary["contactSheetStatus"] = $contactSheetStatus
$summary["timing"] = [ordered]@{
    totalElapsedMs = $timingSummary.totalElapsedMs
    jsonl = $timingPath
    summary = $timingSummaryPath
}
$summary["artifactValidation"] = $artifactValidationStatus
$summary | ConvertTo-Json -Depth 8 | Set-Content -Path $summaryPath -Encoding UTF8

try {
    & python $artifactValidator --suite-root $sessionRoot | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -ne 0) {
        throw "Canvas/custom parity artifact validation failed with exit code $LASTEXITCODE"
    }
    $artifactValidationStatus["status"] = "ok"
} catch {
    $artifactValidationStatus["status"] = "failed"
    $artifactValidationStatus["error"] = $_.Exception.Message
    $summary["artifactValidation"] = $artifactValidationStatus
    $summary | ConvertTo-Json -Depth 8 | Set-Content -Path $summaryPath -Encoding UTF8
    throw
}
$summary["artifactValidation"] = $artifactValidationStatus
$summary | ConvertTo-Json -Depth 8 | Set-Content -Path $summaryPath -Encoding UTF8
$summary | ConvertTo-Json -Depth 7
