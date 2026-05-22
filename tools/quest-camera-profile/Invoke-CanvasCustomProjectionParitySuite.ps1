param(
    [string]$Serial = "",
    [string]$Adb = "adb",
    [string]$Npx = "npx",
    [string]$RunRoot = "artifacts\canvas-custom-projection-parity-suite",
    [string]$HwbApk = "examples\quest-composite-layer-apk\build\outputs\rusty-xr-quest-composite-layer-debug.apk",
    [string]$GlesApk = "examples\quest-gl-openxr-video-stack-apk\build\outputs\rusty-xr-quest-gl-openxr-video-stack-debug.apk",
    [string]$MakepadApk = "examples\makepad-q2q-camera-shell\target\android\makepad-android-apk\rusty_xr_makepad_q2q_camera_shell\apk\rustyx_rmakepadalignment.apk",
    [string]$MakepadPackageName = "com.example.rustyxr.makepad.alignment",
    [int]$WarmupSeconds = 12,
    [int]$MediaProjectionPort = 8787,
    [switch]$Install
)

$ErrorActionPreference = "Stop"

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
$makepadRunner = Join-Path $repoRoot "examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1"

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

function Start-MediaProjectionReceiver {
    param([string]$Dir)
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    $null = Invoke-Adb -Arguments @("reverse", "tcp:$MediaProjectionPort", "tcp:$MediaProjectionPort")
    $stdout = Join-Path $Dir "receiver-stdout.txt"
    $stderr = Join-Path $Dir "receiver-stderr.txt"
    $process = Start-Process `
        -FilePath "python" `
        -ArgumentList @($receiver, "--port", $MediaProjectionPort.ToString(), "--output", $Dir, "--once", "--max-frames", "1") `
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
    if (-not $Process.WaitForExit(45000)) {
        throw "MediaProjection receiver did not receive a frame for $OutputPng. PROJECT_MEDIA app-op pregrant did not produce a capture frame; if a Meta selector is visible in-headset, grant MediaProjection manually and rerun."
    }
    $frames = Join-Path $Dir "frames.jsonl"
    if (-not (Test-Path -LiteralPath $frames)) {
        throw "MediaProjection receiver wrote no frames ledger for $OutputPng"
    }
    & python $converter --frames $frames --output $OutputPng --latest | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -ne 0) {
        throw "MediaProjection PNG conversion failed for $OutputPng"
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
    $latest = Get-ChildItem -LiteralPath $ProfileRoot -Directory |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $latest) {
        throw "No profile run directory found under $ProfileRoot"
    }
    $source = Get-ChildItem -LiteralPath $latest.FullName -Filter "*-hzdb-screencap.png" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $source) {
        throw "HzDB screenshot not found under $($latest.FullName)"
    }
    [System.IO.File]::Copy(
        (ConvertTo-WindowsLongPath $source.FullName),
        (ConvertTo-WindowsLongPath $OutputPng),
        $true)
    return $latest.FullName
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
        $receiverProcess = Start-MediaProjectionReceiver -Dir $mediaRoot
        & powershell @args | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0) {
            throw "$caseId profile run failed"
        }
        Complete-MediaProjectionCapture -Process $receiverProcess -Dir $mediaRoot -OutputPng $mediaPng
        $artifactDir = Copy-HzdbFromProfileRun -ProfileRoot $caseRoot -RuntimeProfile $RuntimeProfile -OutputPng $hzdbPng
        if ($Lane -ne "hwb") {
            Assert-BoundedFootprintEvidence -CaseId $caseId -ArtifactDir $artifactDir
        }
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
    }
}

function Invoke-MakepadCase {
    param(
        [string]$Mode,
        [string]$CameraProjectionMode,
        [string]$ProjectionGeometryProfile
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
    $args = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", $makepadRunner,
        "-Serial", $Serial,
        "-Apk", (Resolve-RepoPath $MakepadApk),
        "-PackageName", $MakepadPackageName,
        "-OutDir", $caseRoot,
        "-StartupTimeoutSeconds", "35",
        "-SampleSeconds", $WarmupSeconds.ToString(),
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
        "-ProjectionAreaOpacity", "1.0",
        "-ProjectionBorderOpacity", "1.0",
        "-ProjectionBorderPolicy", "passthrough-underlay",
        "-EnableNativePassthrough",
        "-MediaProjection",
        "-MediaProjectionPort", $MediaProjectionPort.ToString()
    )
    if (-not $Install) {
        $args += "-SkipInstall"
    }
    try {
        $receiverProcess = Start-MediaProjectionReceiver -Dir $mediaRoot
        & powershell @args | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0) {
            throw "$caseId Makepad run failed"
        }
        Complete-MediaProjectionCapture -Process $receiverProcess -Dir $mediaRoot -OutputPng $mediaPng

        $hzdbArgs = @("-y", "@meta-quest/hzdb", "capture", "screenshot")
        if ($Serial) {
            $hzdbArgs += @("--device", $Serial)
        }
        $hzdbArgs += @("--method", "screencap", "--output", $hzdbPng)
        & $Npx @hzdbArgs | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $hzdbPng)) {
            throw "$caseId HzDB capture failed"
        }
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
    }
}

$hwbCatalog = "examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json"
$glesCatalog = "examples\quest-gl-openxr-video-stack-apk\catalog\rusty-xr-quest-gl-openxr-video-stack.catalog.json"
$records = @()
$records += Invoke-HwbOrGlesCase `
    -Lane "hwb" `
    -Mode "canvas" `
    -Catalog $hwbCatalog `
    -AppId "rusty-xr-quest-composite-layer" `
    -DeviceProfile "xr-composite-comparison-level-5" `
    -RuntimeProfile "camera-stereo-gpu-composite-world-canvas-native-aligned-mediaprojection" `
    -Apk $HwbApk `
    -Override $surfaceOverride
$records += Invoke-HwbOrGlesCase `
    -Lane "hwb" `
    -Mode "custom" `
    -Catalog $hwbCatalog `
    -AppId "rusty-xr-quest-composite-layer" `
    -DeviceProfile "xr-composite-comparison-level-5" `
    -RuntimeProfile "camera-stereo-gpu-composite-camera-footprint-canvas-equivalent-depth1" `
    -Apk $HwbApk `
    -Override ("rustyxr.cameraProjectionMode=display-screen-homography,rustyxr.cameraPipelinePreset=raw-projection-camera-footprint-underlay-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-camera-footprint-underlay,rustyxr.openxrPassthroughProbe=underlay,$surfaceOverride")
$records += Invoke-HwbOrGlesCase `
    -Lane "oes" `
    -Mode "canvas" `
    -Catalog $glesCatalog `
    -AppId "rusty-xr-quest-gl-openxr-video-stack" `
    -DeviceProfile "gles-openxr-comparison-level-5" `
    -RuntimeProfile "gles-direct-camera2-oes-world-canvas-mediaprojection" `
    -Apk $GlesApk `
    -Override ("rustyxr.cameraProjectionMode=world-canvas,rustyxr.directCamera2OesProjectionGeometryProfile=full-frame-diagnostic,rustyxr.projectionBorderPolicy=passthrough-underlay,$surfaceOverride")
$records += Invoke-HwbOrGlesCase `
    -Lane "oes" `
    -Mode "custom" `
    -Catalog $glesCatalog `
    -AppId "rusty-xr-quest-gl-openxr-video-stack" `
    -DeviceProfile "gles-openxr-comparison-level-5" `
    -RuntimeProfile "gles-direct-camera2-oes-camera-projection-mediaprojection" `
    -Apk $GlesApk `
    -Override ("rustyxr.cameraProjectionMode=display-screen-homography,rustyxr.directCamera2OesProjectionGeometryProfile=camera-projection,rustyxr.projectionBorderPolicy=passthrough-underlay,$surfaceOverride")
$records += Invoke-MakepadCase -Mode "canvas" -CameraProjectionMode "world-canvas" -ProjectionGeometryProfile "full-frame-diagnostic"
$records += Invoke-MakepadCase -Mode "custom" -CameraProjectionMode "display-screen-homography" -ProjectionGeometryProfile "camera-projection"

$contactSheetPath = Join-Path $sessionRoot "canvas-custom-projection-parity-results.png"
& python $contactSheetBuilder --session-root $sessionRoot --output $contactSheetPath | ForEach-Object { Write-Host $_ }
if ($LASTEXITCODE -ne 0) {
    throw "Canvas/custom parity contact sheet generation failed"
}

$summary = [ordered]@{
    schemaVersion = "rusty.xr.canvas-custom-projection-parity-suite.v1"
    capturedAt = (Get-Date).ToString("o")
    serial = $Serial
    sessionRoot = $sessionRoot
    screenshotsRoot = $screenshotsRoot
    contactSheet = $contactSheetPath
    geometry = [ordered]@{
        projectionDepthMeters = 1.434085
        cameraPreviewFovYDegrees = 69.763084
        cameraPreviewOffsetYMeters = -0.168832
        cameraRawOverlayOverscan = 1.0
    }
    captureRouteNotes = @(
        "HWB and GLES/OES MediaProjection captures are app-frame evidence for the rendered camera window.",
        "Makepad MediaProjection currently captures the Makepad Android/window surface rather than the submitted OpenXR compositor layer; use HzDB for Makepad geometry until this capture-route difference is resolved."
    )
    records = $records
}
$summaryPath = Join-Path $sessionRoot "canvas-custom-projection-parity-suite-summary.json"
$summary | ConvertTo-Json -Depth 7 | Set-Content -Path $summaryPath -Encoding UTF8
$summary | ConvertTo-Json -Depth 7
