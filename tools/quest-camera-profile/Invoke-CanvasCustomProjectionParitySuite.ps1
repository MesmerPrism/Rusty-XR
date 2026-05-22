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
    [int]$MediaProjectionMaxFrames = 0,
    [int]$MediaProjectionDrainMs = 1500,
    [ValidateSet("direct-camera", "broker-camera")]
    [string]$SourceMode = "direct-camera",
    [ValidateSet("passthrough-underlay", "solid-red")]
    [string]$ProjectionBorderPolicy = "passthrough-underlay",
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

function Get-BrokerH264Override {
    param([string]$ProjectionGeometryProfile)
    return @(
        "rustyxr.brokerH264SourceMode=broker-camera",
        ("rustyxr.brokerH264ProjectionGeometryProfile={0}" -f $ProjectionGeometryProfile),
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
    if ($ProjectionBorderPolicy -eq "solid-red") {
        return "rustyxr.cameraPipelinePreset=raw-projection-solid-red-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-solid-red,rustyxr.openxrPassthroughProbe=off,$projectionOpacityOverride"
    }
    if ($Mode -eq "custom") {
        return "rustyxr.cameraPipelinePreset=raw-projection-camera-footprint-underlay-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-camera-footprint-underlay,rustyxr.openxrPassthroughProbe=underlay,$projectionOpacityOverride"
    }
    return "rustyxr.cameraPipelinePreset=raw-projection-underlay-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-underlay,rustyxr.openxrPassthroughProbe=underlay,$projectionOpacityOverride"
}

function Get-GlesProjectionStyleOverride {
    return $projectionOpacityOverride
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
    if ($SourceMode -ne "broker-camera") {
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
        Restart-BrokerForCase -CaseId $caseId
        $receiverProcess = Start-MediaProjectionReceiver -Dir $mediaRoot
        & powershell @args | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0) {
            throw "$caseId profile run failed"
        }
        Complete-MediaProjectionCapture -Process $receiverProcess -Dir $mediaRoot -OutputPng $mediaPng
        $artifactDir = Copy-HzdbFromProfileRun -ProfileRoot $caseRoot -RuntimeProfile $RuntimeProfile -OutputPng $hzdbPng
        if (($Lane -ne "hwb") -or ($Mode -eq "custom")) {
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
        "-ProjectionAreaOpacity", $ProjectionAreaOpacity.ToString("0.######", [System.Globalization.CultureInfo]::InvariantCulture),
        "-ProjectionBorderOpacity", $ProjectionBorderOpacity.ToString("0.######", [System.Globalization.CultureInfo]::InvariantCulture),
        "-ProjectionBorderPolicy", $ProjectionBorderPolicy,
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
$BrokerClientPackages = @(
    (Get-CatalogPackageName -Catalog $hwbCatalog),
    (Get-CatalogPackageName -Catalog $glesCatalog),
    $MakepadPackageName
) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique
$hwbCanvasRuntimeProfile = if ($SourceMode -eq "broker-camera") {
    "broker-h264-stereo-live-world-canvas-mediaprojection"
}
else {
    "camera-stereo-gpu-composite-world-canvas-native-aligned-mediaprojection"
}
$hwbCustomRuntimeProfile = if ($SourceMode -eq "broker-camera") {
    "broker-h264-stereo-live-openxr-projection-full-feed-control"
}
else {
    "camera-stereo-gpu-composite-camera-footprint-canvas-equivalent-depth1"
}
$glesCanvasRuntimeProfile = if ($SourceMode -eq "broker-camera") {
    "gles-broker-camera-h264-oes-projection"
}
else {
    "gles-direct-camera2-oes-world-canvas-mediaprojection"
}
$glesCustomRuntimeProfile = if ($SourceMode -eq "broker-camera") {
    "gles-broker-camera-h264-oes-projection"
}
else {
    "gles-direct-camera2-oes-camera-projection-mediaprojection"
}
$canvasProjectionAreaOverride = if ($BoundedCanvasProjectionArea) { $boundedProjectionAreaOverride } else { "" }
$hwbCanvasSourceOverride = if ($SourceMode -eq "broker-camera") { Get-BrokerH264Override -ProjectionGeometryProfile "full-frame-diagnostic" } else { "" }
$hwbCustomSourceOverride = if ($SourceMode -eq "broker-camera") { Get-BrokerH264Override -ProjectionGeometryProfile "camera-projection" } else { "" }
$glesCanvasSourceOverride = if ($SourceMode -eq "broker-camera") { Get-BrokerH264Override -ProjectionGeometryProfile "full-frame-diagnostic" } else { "" }
$glesCustomSourceOverride = if ($SourceMode -eq "broker-camera") { Get-BrokerH264Override -ProjectionGeometryProfile "camera-projection" } else { "" }
$records = @()
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
$records += Invoke-MakepadCase -Mode "canvas" -CameraProjectionMode "world-canvas" -ProjectionGeometryProfile "full-frame-diagnostic" -CaseSourceMode $SourceMode
$records += Invoke-MakepadCase -Mode "custom" -CameraProjectionMode "display-screen-homography" -ProjectionGeometryProfile "camera-projection" -CaseSourceMode $SourceMode

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
        projectionAreaOpacity = $ProjectionAreaOpacity
        projectionBorderOpacity = $ProjectionBorderOpacity
        boundedCanvasProjectionArea = [bool]$BoundedCanvasProjectionArea
        boundedProjectionAreaRadiusXUv = 0.47
        boundedProjectionAreaRadiusYUv = 0.36
        boundedProjectionAreaCornerRadiusUv = 0.08
    }
    captureRouteNotes = @(
        "HWB and GLES/OES MediaProjection captures are latest-frame app-frame evidence for the rendered camera window after the profile run.",
        "Makepad MediaProjection currently captures the Makepad Android/window surface rather than the submitted OpenXR compositor layer; use HzDB for Makepad geometry until this capture-route difference is resolved.",
        "Broker-camera runs restart the broker service before each condition and request physical Camera2 H.264 streams with explicit projection metadata."
    )
    records = $records
}
$summaryPath = Join-Path $sessionRoot "canvas-custom-projection-parity-suite-summary.json"
$summary | ConvertTo-Json -Depth 7 | Set-Content -Path $summaryPath -Encoding UTF8
$summary | ConvertTo-Json -Depth 7
