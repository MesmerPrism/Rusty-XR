param(
    [string]$Serial = "",
    [string]$Adb = "adb",
    [string]$Npx = "npx",
    [string]$RunRoot = "artifacts\canvas-custom-projection-parity-suite",
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

$receiver = Join-Path $repoRoot "tools\media-pipeline\frame_receiver.py"
$converter = Join-Path $repoRoot "tools\media-pipeline\Convert-RgbaFrameToPng.py"
$contactSheetBuilder = Join-Path $repoRoot "tools\quest-camera-profile\Build-CanvasCustomParityContactSheet.py"
$profileRunner = Join-Path $repoRoot "tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1"
$makepadRunner = Join-Path $repoRoot "examples\makepad-camera-shell\tools\Invoke-MakepadCameraDeviceGate.ps1"

$surfaceOverride = @(
    "rustyxr.projectionDepthMeters=1.434085",
    "rustyxr.cameraPreviewFovYDegrees=69.763084",
    "rustyxr.cameraPreviewOffsetYMeters=-0.168832",
    "rustyxr.cameraRawOverlayOverscan=1.0",
    "rustyxr.mediaProjection=true",
    ("rustyxr.mediaProjectionPort={0}" -f $MediaProjectionPort),
    "rustyxr.mediaProjectionWidth=512",
    "rustyxr.mediaProjectionHeight=288"
) -join ","

$boundedProjectionAreaOverride = @(
    "rustyxr.projectionAreaOffsetXUv=0.0",
    "rustyxr.projectionAreaOffsetYUv=0.0",
    "rustyxr.projectionAreaScaleUv=1.0",
    "rustyxr.projectionAreaRadiusXUv=0.47",
    "rustyxr.projectionAreaRadiusYUv=0.36",
    "rustyxr.projectionAreaCornerRadiusUv=0.08"
) -join ","

$projectionOpacityOverride = @(
    ("rustyxr.projectionAreaOpacity={0}" -f $ProjectionAreaOpacity.ToString("0.######", [System.Globalization.CultureInfo]::InvariantCulture)),
    ("rustyxr.projectionBorderOpacity={0}" -f $ProjectionBorderOpacity.ToString("0.######", [System.Globalization.CultureInfo]::InvariantCulture)),
    ("rustyxr.projectionBorderPolicy={0}" -f $ProjectionBorderPolicy)
) -join ","
$blurRadiusPxText = $BlurRadiusPx.ToString("0.0#####", [System.Globalization.CultureInfo]::InvariantCulture)
$processingLayerOverride = @(
    ("rustyxr.processingLayer={0}" -f $ProcessingLayer),
    ("rustyxr.cameraBlurRadiusPx={0}" -f $blurRadiusPxText)
) -join ","
$ExpectedMakepadSourceEyeMapping = "display-left-from-left-source"
$brokerSourceRequested = $SourceMode -eq "broker-camera" -or $SourceMode -eq "broker-synthetic"

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
    Start-Sleep -Milliseconds 900
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
    if ($Serial) {
        & $Adb -s $Serial reverse --remove "tcp:$MediaProjectionPort" 2>$null | Out-Null
    } else {
        & $Adb reverse --remove "tcp:$MediaProjectionPort" 2>$null | Out-Null
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
        throw "MediaProjection receiver did not receive a frame for $OutputPng. PROJECT_MEDIA app-op pregrant did not produce a capture frame; if a Meta selector is visible in-headset, grant MediaProjection manually and rerun."
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
    $stateRoot = Join-Path $CaseRoot "post-hzdb-state"
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
        throw "$CaseId Makepad app error was visible after HzDB capture; see $stateRoot"
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

    $evidence = [ordered]@{
        caseId = $CaseId
        artifactDir = $ArtifactDir
        leftProjectionAreaScreenUvRect = $leftProjectionArea
        rightProjectionAreaScreenUvRect = $rightProjectionArea
        leftExpectedSourceValidScreenUvRect = $leftExpected
        rightExpectedSourceValidScreenUvRect = $rightExpected
        projectionAreaFullscreen = (Test-FullscreenRect $leftProjectionArea) -or (Test-FullscreenRect $rightProjectionArea)
        effectiveFootprintFullscreen = (Test-FullscreenRect $leftExpected) -or (Test-FullscreenRect $rightExpected)
    }
    $evidencePath = Join-Path $sessionRoot ("bounded-footprint-evidence-$CaseId.json")
    $evidence | ConvertTo-Json -Depth 5 | Set-Content -Path $evidencePath -Encoding UTF8

    if ($null -eq $leftExpected -or $null -eq $rightExpected) {
        throw "[$CaseId] no left/right expected source-valid footprint was logged; cannot prove bounded geometry."
    }
    if ((Test-FullscreenRect $leftExpected) -or (Test-FullscreenRect $rightExpected)) {
        throw "[$CaseId] effective source-valid footprint is fullscreen; rejecting parity evidence. See $evidencePath"
    }
    if ((Test-FullscreenRect $leftProjectionArea) -or (Test-FullscreenRect $rightProjectionArea)) {
        Write-Warning "[$CaseId] shader projectionAreaScreenUvRect is fullscreen; accepting only because effective source-valid footprint is bounded. See $evidencePath"
    }
}

function Copy-HzdbFromProfileRun {
    param(
        [string]$ProfileRoot,
        [string]$RuntimeProfile,
        [string]$OutputPng
    )
    $lastError = $null
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        try {
            $latest = Get-ChildItem -LiteralPath (ConvertTo-WindowsLongPath $ProfileRoot) -Directory -ErrorAction Stop |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if (-not $latest) {
                $lastError = "No profile run directory found under $ProfileRoot"
            } else {
                $source = Get-ChildItem -LiteralPath (ConvertTo-WindowsLongPath $latest.FullName) -Filter "*-hzdb-screencap.png" -ErrorAction Stop |
                    Sort-Object LastWriteTime -Descending |
                    Select-Object -First 1
                if ($source) {
                    [System.IO.File]::Copy(
                        (ConvertTo-WindowsLongPath $source.FullName),
                        (ConvertTo-WindowsLongPath $OutputPng),
                        $true)
                    return $latest.FullName
                }
                $lastError = "HzDB screenshot not found under $($latest.FullName)"
            }
        } catch {
            $lastError = $_.Exception.Message
        }
        Start-Sleep -Milliseconds 500
    }
    throw $lastError
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
    Write-Host "[$caseId] MediaProjection receiver starting"
    $caseRoot = Join-Path $profileRunRoot $caseId
    $mediaRoot = Join-Path $sessionRoot "mediaprojection\$caseId"
    $mediaPng = Join-Path $screenshotsRoot "$caseId-mediaprojection.png"
    $hzdbPng = Join-Path $screenshotsRoot "$caseId-hzdb.png"
    $receiverProcess = $null

    $args = @(
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
        "-CaptureHzdbScreencap",
        "-FreshnessFrames", "1",
        "-SkipProximityHold",
        "-LogcatLines", "16000",
        "-Override", $Override
    )
    if ($Install) {
        $args += @("-Install", "-Apk", (Resolve-RepoPath $Apk))
    }
    try {
        Restart-BrokerForCase -CaseId $caseId
        $receiverProcess = Start-MediaProjectionReceiver -Dir $mediaRoot
        & powershell @args | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0) {
            throw "$caseId profile run failed"
        }
        Complete-MediaProjectionCapture -Process $receiverProcess -Dir $mediaRoot -OutputPng $mediaPng
        $artifactDir = Copy-HzdbFromProfileRun -ProfileRoot $caseRoot -RuntimeProfile $RuntimeProfile -OutputPng $hzdbPng
        Assert-BoundedFootprintEvidence -CaseId $caseId -ArtifactDir $artifactDir
    }
    finally {
        Stop-MediaProjectionReceiver -Process $receiverProcess
        Remove-MediaProjectionReverse
    }
    Write-Host "[$caseId] captured MediaProjection and HzDB"
    return [ordered]@{
        id = $caseId
        lane = $Lane
        mode = $Mode
        runtimeProfile = $RuntimeProfile
        artifactDir = $artifactDir
        mediaProjection = $mediaPng
        hzdb = $hzdbPng
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
    Write-Host "[$caseId] MediaProjection receiver starting"
    $caseRoot = Join-Path $makepadRunRoot $caseId
    $mediaRoot = Join-Path $sessionRoot "mediaprojection\$caseId"
    $mediaPng = Join-Path $screenshotsRoot "$caseId-mediaprojection.png"
    $hzdbPng = Join-Path $screenshotsRoot "$caseId-hzdb.png"
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
    $args = @(
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
        "-ProjectionAreaRadiusXUv", "0.47",
        "-ProjectionAreaRadiusYUv", "0.36",
        "-ProjectionAreaCornerRadiusUv", "0.08",
        "-ProjectionAreaOpacity", $ProjectionAreaOpacity.ToString("0.######", [System.Globalization.CultureInfo]::InvariantCulture),
        "-ProjectionBorderOpacity", $ProjectionBorderOpacity.ToString("0.######", [System.Globalization.CultureInfo]::InvariantCulture),
        "-ProjectionBorderPolicy", $ProjectionBorderPolicy,
        "-ProcessingLayer", $ProcessingLayer,
        "-BlurRadiusPx", $blurRadiusPxText,
        "-MediaProjection",
        "-MediaProjectionPort", $MediaProjectionPort.ToString()
    )
    if (Get-MakepadNativePassthroughRequested) {
        $args += "-EnableNativePassthrough"
    }
    if ($CaseSourceMode -eq "broker-camera") {
        $args += @(
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
        $args += @(
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
        $args += "-SkipInstall"
    }
    try {
        Restart-BrokerForCase -CaseId $caseId
        $receiverProcess = Start-MediaProjectionReceiver -Dir $mediaRoot
        & powershell @args | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0) {
            throw "$caseId Makepad run failed"
        }
        Assert-MakepadSourceEyeMapping -CaseId $caseId -CaseRoot $caseRoot
        Wait-MakepadForegroundForHzdb -CaseId $caseId -CaseRoot $caseRoot

        $hzdbArgs = @("-y", "@meta-quest/hzdb", "capture", "screenshot")
        if ($Serial) {
            $hzdbArgs += @("--device", $Serial)
        }
        $hzdbArgs += @("--method", "screencap", "--output", $hzdbPng)
        & $Npx @hzdbArgs | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $hzdbPng)) {
            throw "$caseId HzDB capture failed"
        }
        Assert-MakepadNoApplicationError -CaseId $caseId -CaseRoot $caseRoot
        Complete-MediaProjectionCapture -Process $receiverProcess -Dir $mediaRoot -OutputPng $mediaPng
        Assert-BoundedFootprintEvidence -CaseId $caseId -ArtifactDir $caseRoot
    }
    finally {
        Stop-MediaProjectionReceiver -Process $receiverProcess
        Remove-MediaProjectionReverse
    }
    Write-Host "[$caseId] captured MediaProjection and HzDB"
    return [ordered]@{
        id = $caseId
        lane = "makepad"
        mode = $Mode
        cameraProjectionMode = $CameraProjectionMode
        runtimeProfile = $ProjectionGeometryProfile
        artifactDir = $caseRoot
        mediaProjection = $mediaPng
        hzdb = $hzdbPng
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
else {
    "gles-direct-camera2-oes-world-canvas-mediaprojection"
}
$glesCustomRuntimeProfile = if ($brokerSourceRequested) {
    "gles-broker-camera-h264-oes-projection"
}
else {
    "gles-direct-camera2-oes-camera-projection-mediaprojection"
}
$canvasProjectionAreaOverride = if ($BoundedCanvasProjectionArea) { $boundedProjectionAreaOverride } else { "" }
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
            $canvasProjectionAreaOverride,
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
            $boundedProjectionAreaOverride,
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
            $canvasProjectionAreaOverride,
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
            $boundedProjectionAreaOverride,
            $glesCustomSourceOverride,
            $surfaceOverride
        ))
}
if (Test-LaneEnabled -Lane "makepad") {
    $records += Invoke-MakepadCase -Mode "canvas" -CameraProjectionMode "world-canvas" -ProjectionGeometryProfile "full-frame-diagnostic" -CaseSourceMode $SourceMode
    $records += Invoke-MakepadCase -Mode "custom" -CameraProjectionMode "display-screen-homography" -ProjectionGeometryProfile "camera-projection" -CaseSourceMode $SourceMode
}

$contactSheetPath = Join-Path $sessionRoot "canvas-custom-projection-parity-results.png"
& python $contactSheetBuilder --session-root $sessionRoot --output $contactSheetPath | ForEach-Object { Write-Host $_ }
if ($LASTEXITCODE -ne 0) {
    throw "Canvas/custom parity contact sheet generation failed"
}

$summary = [ordered]@{
    schemaVersion = "rusty.xr.canvas-custom-projection-parity-suite.v1"
    capturedAt = (Get-Date).ToString("o")
    serial = $Serial
    sourceMode = $SourceMode
    sessionRoot = $sessionRoot
    screenshotsRoot = $screenshotsRoot
    contactSheet = $contactSheetPath
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
        boundedProjectionAreaRadiusXUv = 0.47
        boundedProjectionAreaRadiusYUv = 0.36
        boundedProjectionAreaCornerRadiusUv = 0.08
        makepadStartupTimeoutSeconds = $MakepadStartupTimeoutSeconds
        makepadSampleSeconds = [Math]::Max($MakepadSampleSeconds, $WarmupSeconds)
        makepadPostRunSettleSeconds = $MakepadPostRunSettleSeconds
        expectedMakepadSourceEyeMapping = $ExpectedMakepadSourceEyeMapping
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
        "HWB and GLES/OES MediaProjection captures are latest-frame app-frame evidence for the rendered camera window after the profile run.",
        "Makepad MediaProjection currently captures the Makepad Android/window surface rather than the submitted OpenXR compositor layer; use HzDB for Makepad geometry until this capture-route difference is resolved.",
        "MediaProjection is display/app-window mirror evidence and may visually align with a different HzDB eye by renderer; do not use its apparent eye index as source-eye parity proof.",
        "Broker-source runs restart the broker service before each condition. Broker-camera requests physical Camera2 H.264 streams; broker-synthetic requests the diagnostic H.264 stimulus with camera-matched metadata for synthetic blur evidence."
    )
    records = $records
}
$summaryPath = Join-Path $sessionRoot "canvas-custom-projection-parity-suite-summary.json"
$summary | ConvertTo-Json -Depth 7 | Set-Content -Path $summaryPath -Encoding UTF8
$summary | ConvertTo-Json -Depth 7
