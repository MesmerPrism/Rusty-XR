param(
    [Parameter(Mandatory = $true)]
    [string]$Serial,

    [Parameter(Mandatory = $true)]
    [string]$Apk,

    [string]$PackageName = "io.github.mesmerprism.rustyquest.makepad.camera",
    [string]$LauncherActivity = ("." + "Makepad" + "App"),
    [string]$XrActivity = ("." + "Makepad" + "App" + "Xr"),
    [string]$OutDir = "",
    [int]$StartupTimeoutSeconds = 30,
    [int]$SampleSeconds = 90,
    [int]$ReadyPollIntervalMs = 750,
    [int]$ReadySettleMs = 1500,
    [switch]$UseFixedSampleWindow,
    [int]$FreshnessFrames = 6,
    [int]$FreshnessIntervalSeconds = 1,
    [int]$FreshnessRequiredUniqueHashes = 1,
    [int]$BrokerH264ReadyTimeoutSeconds = 30,
    [switch]$SkipInstall,
    [switch]$SkipDirectXrFallback,
    [switch]$PreferDirectVrActivity,
    [switch]$UseBrokerH264Synthetic,
    [switch]$UseBrokerH264Camera,
    [string]$BrokerH264Host = "127.0.0.1",
    [int]$BrokerH264BrokerPort = 8765,
    [int]$BrokerH264LeftStreamPort = 8879,
    [int]$BrokerH264RightStreamPort = 8880,
    [string]$BrokerH264SyntheticPattern = "diagnostic-grid",
    [string]$BrokerH264ProjectionGeometryProfile = "",
    [ValidateSet("display-screen-homography", "world-canvas")]
    [string]$CameraProjectionMode = "display-screen-homography",
    [ValidateSet("full-frame-diagnostic", "camera-projection")]
    [string]$CameraProjectionGeometryProfile = "camera-projection",
    [ValidateSet("target-local-raster", "screen-to-camera-homography")]
    [string]$CameraSourceSamplingMode = "target-local-raster",
    [string]$CameraTargetScreenUvRect = "",
    [string]$CameraLeftTargetScreenUvRect = "0.171875;0.21875;0.75;0.65625",
    [string]$CameraRightTargetScreenUvRect = "0.078125;0.21875;0.75;0.671875",
    [ValidateSet("head-anchored-virtual-camera", "camera-matched", "full-frame-diagnostic")]
    [string]$BrokerH264SyntheticProjectionProfile = "head-anchored-virtual-camera",
    [ValidateSet("cpu-yuv", "hardware-buffer", "surface-texture")]
    [string]$BrokerH264DecodeOutputMode = "cpu-yuv",
    [string]$BrokerH264LeftCameraId = "50",
    [string]$BrokerH264RightCameraId = "51",
    [int]$BrokerH264Width = 1280,
    [int]$BrokerH264Height = 1280,
    [int]$BrokerH264CaptureMs = 45000,
    [int]$BrokerH264MaxPackets = 0,
    [int]$BrokerH264BitrateBps = 6000000,
    [int]$BrokerH264FrameRateHz = 50,
    [string]$BrokerH264StereoPairId = "makepad-broker-h264-stereo-camera",
    [int]$BrokerH264StereoPairMaxDeltaNs = 25000000,
    [int]$BrokerH264StreamTimeoutMs = 60000,
    [int]$BrokerH264DecodeTimeoutMs = 20000,
    [switch]$RequireBrokerH264StereoProjection,
    [int]$BrokerH264MinimumPerEyeTextureUpdates = 1,
    [ValidateSet("solid-red", "passthrough-underlay")]
    [string]$ProjectionBorderPolicy = "solid-red",
    [ValidateSet("raw", "blur", "peripheral-stretch")]
    [string]$ProcessingLayer = "peripheral-stretch",
    [ValidateSet("camera", "solid-color", "solid-no-texture", "clear-only")]
    [string]$ProjectionSampleMode = "camera",
    [double]$BlurRadiusPx = 2.0,
    [ValidateSet("edge-stretch")]
    [string]$PeripheralStretchMode = "edge-stretch",
    [double]$PeripheralStretchCoreScale = 1.0,
    [double]$PeripheralStretchEdgeInsetUv = 0.015,
    [double]$PeripheralStretchMaxInsetUv = 0.14,
    [double]$PeripheralStretchCurve = 1.6,
    [double]$PeripheralStretchInnerBlendUv = 0.040,
    [double]$PeripheralStretchBlendCurve = 1.6,
    [ValidateSet("off", "target-inner-band")]
    [string]$PeripheralStretchBlendMode = "target-inner-band",
    [ValidateSet("target-footprint")]
    [string]$PeripheralStretchCornerMode = "target-footprint",
    [ValidateSet("off", "regions", "sample-uv")]
    [string]$PeripheralStretchDebug = "off",
    [double]$ProjectionScale = 1.0,
    [double]$ProjectionDepthMeters = 1.434085,
    [double]$CameraPreviewFovYDegrees = 69.763084,
    [double]$CameraPreviewOffsetYMeters = -0.168832,
    [double]$CameraRawOverlayOverscan = 1.0,
    [double]$XrRenderScale = 0.90,
    [double]$XrDisplayRefreshHz = 72.0,
    [ValidateRange(0, 5)]
    [int]$OculusCpuLevel = 4,
    [ValidateRange(0, 5)]
    [int]$OculusGpuLevel = 4,
    [ValidateRange(0, 4)]
    [int]$OculusFoveationLevel = 0,
    [ValidateSet("true", "false")]
    [string]$OculusFoveationDynamic = "false",
    [double]$ProjectionAreaOffsetXUv = 0.0,
    [double]$ProjectionAreaOffsetLeftUv = [double]::NaN,
    [double]$ProjectionAreaOffsetRightUv = [double]::NaN,
    [double]$ProjectionAreaOffsetYUv = 0.0,
    [double]$ProjectionAreaScaleX = 1.0,
    [double]$ProjectionAreaScaleY = 1.0,
    [double]$ProjectionTargetOffsetXUv = 0.0,
    [double]$ProjectionTargetOffsetYUv = 0.0,
    [double]$ProjectionTargetScale = 1.0,
    [ValidateSet("off", "offset-scale")]
    [string]$ProjectionTargetJoystickControls = "offset-scale",
    [double]$ProjectionAreaRadiusXUv = 0.5,
    [double]$ProjectionAreaRadiusYUv = 0.5,
    [double]$ProjectionAreaCornerRadiusUv = 0.0,
    [double]$ProjectionAreaOpacity = 1.0,
    [double]$ProjectionBorderOpacity = 1.0,
    [ValidateRange(0.0, 2.0)]
    [double]$ProjectionAreaDiagnostic = 0.0,
    [ValidateSet("fixed", "red", "green", "blue", "luma", "inverse-red", "inverse-green", "inverse-blue", "inverse-luma", "red-dominance", "green-dominance", "blue-dominance", "saturation", "inverse-saturation")]
    [string]$ProjectionAlphaMode = "fixed",
    [double]$ProjectionAlphaScale = 1.0,
    [double]$ProjectionAlphaBias = 0.0,
    [switch]$UseResolvedProjectionRuntime = $true,
    [switch]$MediaProjection,
    [int]$MediaProjectionPort = 8787,
    [int]$MediaProjectionWidth = 512,
    [int]$MediaProjectionHeight = 288,
    [int]$MediaProjectionDelayMs = 1600,
    [ValidateSet("fail", "clear", "ignore")]
    [string]$ProjectionPropertyHygiene = "clear",
    [ValidateSet("skip", "warn", "required")]
    [string]$ProjectionRuntimeReadback = "warn",
    [ValidateSet("cpu-yuv", "hardware-buffer-external")]
    [string]$DirectCameraTexturePath = "hardware-buffer-external",
    [string[]]$PreLaunchForceStopPackages = @(
        "com.example.rustyxr.composite",
        "com.example.rustyxr.opengles"
    ),
    [switch]$SkipPreLaunchForceStopPackages,
    [switch]$EnableNativePassthrough
)

$ErrorActionPreference = "Stop"
$ProjectionAreaScaleMin = 0.01
$ProjectionAreaScaleMax = 10.0
if ($FreshnessFrames -lt 1) {
    throw "FreshnessFrames must be at least 1."
}
if ($FreshnessIntervalSeconds -lt 0) {
    throw "FreshnessIntervalSeconds must be non-negative."
}
if ($FreshnessRequiredUniqueHashes -lt 1) {
    throw "FreshnessRequiredUniqueHashes must be at least 1."
}
if ($FreshnessRequiredUniqueHashes -gt $FreshnessFrames) {
    throw "FreshnessRequiredUniqueHashes cannot exceed FreshnessFrames."
}
if ($BrokerH264MinimumPerEyeTextureUpdates -lt 1) {
    throw "BrokerH264MinimumPerEyeTextureUpdates must be at least 1."
}
if ($UseBrokerH264Camera) {
    if ([string]::IsNullOrWhiteSpace($BrokerH264LeftCameraId) -or [string]::IsNullOrWhiteSpace($BrokerH264RightCameraId)) {
        throw "Broker camera runs require explicit left/right camera IDs. Use -BrokerH264LeftCameraId 50 -BrokerH264RightCameraId 51 for the Quest stereo pair."
    }
    if ($BrokerH264LeftCameraId.Trim() -eq $BrokerH264RightCameraId.Trim()) {
        throw "Broker camera left/right camera IDs must differ; both were '$($BrokerH264LeftCameraId.Trim())'."
    }
}
if ([double]::IsNaN($ProjectionTargetScale) -or [double]::IsInfinity($ProjectionTargetScale) -or $ProjectionTargetScale -lt 0.05 -or $ProjectionTargetScale -gt 1.50) {
    throw "ProjectionTargetScale must be finite and within [0.05, 1.50]; got $ProjectionTargetScale"
}
if ([double]::IsNaN($ProjectionAreaScaleX) -or [double]::IsInfinity($ProjectionAreaScaleX) -or $ProjectionAreaScaleX -lt $ProjectionAreaScaleMin -or $ProjectionAreaScaleX -gt $ProjectionAreaScaleMax) {
    throw "ProjectionAreaScaleX must be finite and within [$ProjectionAreaScaleMin, $ProjectionAreaScaleMax]; got $ProjectionAreaScaleX"
}
if ([double]::IsNaN($ProjectionAreaScaleY) -or [double]::IsInfinity($ProjectionAreaScaleY) -or $ProjectionAreaScaleY -lt $ProjectionAreaScaleMin -or $ProjectionAreaScaleY -gt $ProjectionAreaScaleMax) {
    throw "ProjectionAreaScaleY must be finite and within [$ProjectionAreaScaleMin, $ProjectionAreaScaleMax]; got $ProjectionAreaScaleY"
}
if ([double]::IsNaN($ProjectionTargetOffsetXUv) -or [double]::IsInfinity($ProjectionTargetOffsetXUv) -or $ProjectionTargetOffsetXUv -lt -0.5 -or $ProjectionTargetOffsetXUv -gt 0.5) {
    throw "ProjectionTargetOffsetXUv must be finite and within [-0.5, 0.5]; got $ProjectionTargetOffsetXUv"
}
if ([double]::IsNaN($ProjectionTargetOffsetYUv) -or [double]::IsInfinity($ProjectionTargetOffsetYUv) -or $ProjectionTargetOffsetYUv -lt -0.5 -or $ProjectionTargetOffsetYUv -gt 0.5) {
    throw "ProjectionTargetOffsetYUv must be finite and within [-0.5, 0.5]; got $ProjectionTargetOffsetYUv"
}
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\..\.."))
$projectionPropertyHygieneHelper = Join-Path $repoRoot "tools\quest-camera-profile\ProjectionPropertyHygiene.ps1"
$projectionRuntimeReadbackValidator = Join-Path $repoRoot "tools\quest-camera-profile\Validate-ProjectionRuntimeReadback.py"
$freshnessAnalyzer = Join-Path $repoRoot "tools\quest-camera-profile\Analyze-ScreenshotFreshness.py"
$metaPerfStaleAnalyzer = Join-Path $repoRoot "tools\quest-camera-profile\Analyze-MetaPerfStale.py"
$cameraTextureLaneContractBuilder = Join-Path $repoRoot "tools\quest-camera-profile\Build-CameraTextureLaneContracts.py"
$publicExampleAppHygieneHelper = Join-Path $repoRoot "tools\quest-camera-profile\PublicExampleAppHygiene.ps1"
. $projectionPropertyHygieneHelper
. $publicExampleAppHygieneHelper

function Invoke-Adb {
    param([string[]]$Arguments)
    & adb -s $Serial @Arguments
}

function Join-NativeProcessArguments {
    param([string[]]$Arguments)
    $quoted = @()
    foreach ($arg in $Arguments) {
        $text = [string]$arg
        if ($text -match '[\s"]') {
            $text = '"' + ($text -replace '"', '\"') + '"'
        }
        $quoted += $text
    }
    return ($quoted -join " ")
}

function Save-Adb {
    param(
        [string[]]$Arguments,
        [string]$Path,
        [int]$TimeoutSeconds = 0
    )
    if ($TimeoutSeconds -le 0) {
        Invoke-Adb -Arguments $Arguments 2>&1 | Set-Content -Path $Path -Encoding UTF8
        return
    }

    $processInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $processInfo.FileName = "adb"
    $processInfo.UseShellExecute = $false
    $processInfo.RedirectStandardOutput = $true
    $processInfo.RedirectStandardError = $true
    $processInfo.CreateNoWindow = $true
    $processInfo.Arguments = Join-NativeProcessArguments -Arguments (@("-s", $Serial) + $Arguments)

    $process = [System.Diagnostics.Process]::Start($processInfo)
    $timedOut = $false
    try {
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $timedOut = $true
            try {
                $process.Kill($true)
            }
            catch {
                $process.Kill()
            }
            $process.WaitForExit()
        }
        $output = @()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        if (-not [string]::IsNullOrWhiteSpace($stdout)) {
            $output += $stdout.TrimEnd()
        }
        if (-not [string]::IsNullOrWhiteSpace($stderr)) {
            $output += $stderr.TrimEnd()
        }
        if ($timedOut) {
            $output += "adb command timed out after ${TimeoutSeconds}s"
        }
        elseif ($process.ExitCode -ne 0) {
            $output += "adb command exited with code $($process.ExitCode)"
        }
        $output | Set-Content -Path $Path -Encoding UTF8
    }
    finally {
        $process.Dispose()
    }
}

function Convert-ToLongLiteralPath {
    param([string]$Path)
    if ([System.IO.Path]::DirectorySeparatorChar -ne '\') {
        return $Path
    }
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    if ($fullPath.StartsWith("\\?\")) {
        return $fullPath
    }
    if ($fullPath.StartsWith("\\")) {
        return "\\?\UNC\" + $fullPath.Substring(2)
    }
    return "\\?\" + $fullPath
}

function Receive-AdbFile {
    param(
        [string]$Remote,
        [string]$Local
    )
    $tempPath = Join-Path ([System.IO.Path]::GetTempPath()) ("rustyquest-makepad-{0}.tmp" -f [guid]::NewGuid())
    try {
        Invoke-Adb -Arguments @("pull", $Remote, $tempPath) | Out-Null
        Move-Item -LiteralPath $tempPath -Destination (Convert-ToLongLiteralPath -Path $Local) -Force
    }
    finally {
        if (Test-Path -LiteralPath $tempPath) {
            Remove-Item -LiteralPath $tempPath -Force
        }
    }
}

function Activity-Component {
    param([string]$Activity)
    if ($Activity.StartsWith(".")) {
        return "$PackageName/$Activity"
    }
    if ($Activity.Contains("/")) {
        return $Activity
    }
    return "$PackageName/$Activity"
}

function Parse-DoubleInvariant {
    param([string]$Value)
    $parsed = 0.0
    if ([double]::TryParse(
            $Value,
            [System.Globalization.NumberStyles]::Float,
            [System.Globalization.CultureInfo]::InvariantCulture,
            [ref]$parsed)) {
        return $parsed
    }
    return 0.0
}

function Format-InvariantDouble {
    param([double]$Value)
    return $Value.ToString("0.######", [System.Globalization.CultureInfo]::InvariantCulture)
}

function ConvertTo-AndroidShellSingleQuoted {
    param([string]$Value)
    $escaped = $Value.Replace("'", "'\''")
    return "'$escaped'"
}

function Set-AdbProperty {
    param(
        [string]$Name,
        [object]$Value
    )
    $valueText = [string]$Value
    $quotedValue = ConvertTo-AndroidShellSingleQuoted -Value $valueText
    Invoke-Adb -Arguments @("shell", "setprop $Name $quotedValue") | Out-Null
}

function Add-GateTimingRecord {
    param([object]$Record)
    if ($null -eq $script:gateTimingRecords) {
        return
    }
    $script:gateTimingRecords.Add($Record)
    $Record | ConvertTo-Json -Depth 6 -Compress | Add-Content -Path $script:gateTimingPath -Encoding UTF8
}

function Invoke-GateTimedStep {
    param(
        [string]$Step,
        [scriptblock]$Action
    )
    $startedAt = Get-Date
    $startedElapsedMs = if ($script:gateStopwatch) { $script:gateStopwatch.ElapsedMilliseconds } else { 0 }
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
        $endedElapsedMs = if ($script:gateStopwatch) { $script:gateStopwatch.ElapsedMilliseconds } else { 0 }
        Add-GateTimingRecord -Record ([ordered]@{
            step = $Step
            status = $status
            startedAt = $startedAt.ToString("o")
            endedAt = $endedAt.ToString("o")
            startElapsedMs = $startedElapsedMs
            endElapsedMs = $endedElapsedMs
            durationMs = $endedElapsedMs - $startedElapsedMs
            error = $errorMessage
        })
        Write-Host ("[makepad-timing] {0} {1}ms {2}" -f $Step, ($endedElapsedMs - $startedElapsedMs), $status)
    }
}

function Get-GateTimingRecordValue {
    param(
        [object]$Record,
        [string]$Name
    )
    if ($Record -is [System.Collections.IDictionary]) {
        return $Record[$Name]
    }
    return $Record.$Name
}

function New-GateTimingSummary {
    $records = @($script:gateTimingRecords)
    $byStep = @(
        @($records | ForEach-Object { Get-GateTimingRecordValue -Record $_ -Name "step" } | Sort-Object -Unique) |
            ForEach-Object {
                $stepName = [string]$_
                $group = @($records | Where-Object { (Get-GateTimingRecordValue -Record $_ -Name "step") -eq $stepName })
                $durations = @($group | ForEach-Object { [int64](Get-GateTimingRecordValue -Record $_ -Name "durationMs") })
                $sum = ($durations | Measure-Object -Sum).Sum
                $min = ($durations | Measure-Object -Minimum).Minimum
                $max = ($durations | Measure-Object -Maximum).Maximum
                $avg = ($durations | Measure-Object -Average).Average
                [ordered]@{
                    step = $stepName
                    count = $group.Count
                    totalMs = if ($null -ne $sum) { $sum } else { 0 }
                    minMs = if ($null -ne $min) { $min } else { 0 }
                    maxMs = if ($null -ne $max) { $max } else { 0 }
                    avgMs = if ($null -ne $avg) { [Math]::Round($avg, 2) } else { 0.0 }
                    failures = @($group | Where-Object { (Get-GateTimingRecordValue -Record $_ -Name "status") -ne "ok" }).Count
                }
            }
    )
    return [ordered]@{
        schemaVersion = "rusty.quest.makepad-camera-device-gate.timing.v1"
        totalElapsedMs = if ($script:gateStopwatch) { $script:gateStopwatch.ElapsedMilliseconds } else { 0 }
        timingJsonl = $script:gateTimingPath
        records = $records
        byStep = $byStep
    }
}

function Assert-PropertyReadback {
    param(
        [object[]]$Readback,
        [string]$Label
    )
    $mismatches = @($Readback | Where-Object { [string]$_.expected -ne [string]$_.actual })
    if ($mismatches.Count -eq 0) {
        return
    }
    $path = Join-Path $OutDir "$Label-property-mismatches.json"
    $mismatches | ConvertTo-Json -Depth 4 | Set-Content -Path $path -Encoding UTF8
    throw "$Label property readback mismatch; see $path"
}

function Get-ProjectionRuntimeNumericTypeIssues {
    param([string[]]$LogLines)
    $numericKeys = @(
        "projection_scale",
        "projection_depth_meters",
        "camera_projection_fov_y_degrees",
        "camera_preview_fov_y_degrees",
        "camera_preview_offset_y_meters",
        "camera_raw_overlay_overscan",
        "projection_area_scale_uv",
        "projection_area_scale_x",
        "projection_area_scale_y",
        "projection_area_offset_x_uv",
        "projection_area_offset_y_uv",
        "projection_area_left_offset_x_uv",
        "projection_area_left_offset_y_uv",
        "projection_area_right_offset_x_uv",
        "projection_area_right_offset_y_uv",
        "projection_area_radius_x_uv",
        "projection_area_radius_y_uv",
        "projection_area_corner_radius_uv",
        "projection_area_opacity",
        "projection_border_opacity",
        "projection_alpha_scale",
        "projection_alpha_bias",
        "source_visible_rect_x_uv",
        "source_visible_rect_y_uv",
        "source_visible_rect_width_uv",
        "source_visible_rect_height_uv"
    )
    $issues = New-Object System.Collections.Generic.List[string]
    foreach ($line in $LogLines) {
        if ($line -notmatch "RUSTY_QUEST_MAKEPAD_PROJECTION_RUNTIME_MANIFEST") {
            continue
        }
        foreach ($key in $numericKeys) {
            $pattern = [regex]::Escape($key) + "\[[^\]]*resolved=bool:"
            if ($line -match $pattern) {
                $issues.Add($line.Trim())
                break
            }
        }
    }
    return @($issues | Select-Object -Unique)
}

function Invoke-ProjectionRuntimeReadbackValidation {
    param(
        [object[]]$Attempts,
        [string]$Mode
    )

    $outPath = Join-Path $OutDir "projection-runtime-readback.json"
    $stdoutPath = Join-Path $OutDir "projection-runtime-readback-stdout.txt"
    $errorPath = Join-Path $OutDir "projection-runtime-readback-error.txt"

    if ($Mode -eq "skip") {
        $skipped = [ordered]@{
            schemaVersion = "rusty.quest.makepad.projection-runtime-readback.v1"
            status = "skipped"
            mode = $Mode
            report = $outPath
            error = ""
        }
        $skipped | ConvertTo-Json -Depth 5 | Set-Content -Path $outPath -Encoding UTF8
        return $skipped
    }

    if (-not (Test-Path -LiteralPath $projectionRuntimeReadbackValidator)) {
        $missing = [ordered]@{
            schemaVersion = "rusty.quest.makepad.projection-runtime-readback.v1"
            status = "failed"
            mode = $Mode
            report = $outPath
            error = "projection runtime readback validator not found: $projectionRuntimeReadbackValidator"
        }
        $missing | ConvertTo-Json -Depth 5 | Set-Content -Path $outPath -Encoding UTF8
        return $missing
    }

    $logcatPaths = @()
    foreach ($attempt in $Attempts) {
        if ($null -eq $attempt -or [string]::IsNullOrWhiteSpace([string]$attempt.label)) {
            continue
        }
        $candidate = Join-Path $OutDir (Join-Path ([string]$attempt.label) "logcat.txt")
        if (Test-Path -LiteralPath $candidate) {
            $logcatPaths += $candidate
        }
    }
    if ($logcatPaths.Count -eq 0) {
        $missingLogs = [ordered]@{
            schemaVersion = "rusty.quest.makepad.projection-runtime-readback.v1"
            status = "failed"
            mode = $Mode
            report = $outPath
            error = "no Makepad launch logcat files were available for projection runtime readback validation"
        }
        $missingLogs | ConvertTo-Json -Depth 5 | Set-Content -Path $outPath -Encoding UTF8
        return $missingLogs
    }

    $validatorArgs = @(
        $projectionRuntimeReadbackValidator,
        "--expected-source", "android-property",
        "--expected-backend", "makepad",
        "--out", $outPath
    )
    foreach ($propertyPath in @(
            (Join-Path $OutDir "projection-target-props.json"),
            (Join-Path $OutDir "broker-h264-props.json")
        )) {
        if (Test-Path -LiteralPath $propertyPath) {
            $validatorArgs += @("--expected-properties", $propertyPath)
        }
    }
    foreach ($logcatPath in ($logcatPaths | Sort-Object -Unique)) {
        $validatorArgs += @("--logcat", $logcatPath)
    }
    if ($Mode -eq "warn") {
        $validatorArgs += "--allow-missing-manifest"
    }

    try {
        $output = @(& python @validatorArgs 2>&1 | ForEach-Object { [string]$_ })
        $exitCode = $LASTEXITCODE
        $output | Set-Content -Path $stdoutPath -Encoding UTF8
        if ($exitCode -ne 0) {
            "projection runtime readback validation failed with exit code $exitCode; see $outPath" |
                Set-Content -Path $errorPath -Encoding UTF8
        }
    }
    catch {
        @("projection runtime readback validation failed", $_.Exception.Message) |
            Set-Content -Path $errorPath -Encoding UTF8
    }

    if (Test-Path -LiteralPath $outPath) {
        try {
            return Get-Content -Raw -LiteralPath $outPath | ConvertFrom-Json
        }
        catch {
        }
    }
    return [ordered]@{
        schemaVersion = "rusty.quest.makepad.projection-runtime-readback.v1"
        status = "failed"
        mode = $Mode
        report = $outPath
        error = "projection runtime readback report was not written or readable"
    }
}

function Get-Sha256Hex {
    param([string]$Path)
    if (Get-Command -Name Get-FileHash -ErrorAction SilentlyContinue) {
        return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash
    }

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            return ([System.BitConverter]::ToString($sha.ComputeHash($stream)) -replace "-", "")
        }
        finally {
            $sha.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Grant-RuntimePermissions {
    $permissions = @(
        "android.permission.CAMERA",
        "android.permission.RECORD_AUDIO",
        "com.oculus.permission.USE_SCENE",
        "horizonos.permission.USE_SCENE",
        "horizonos.permission.HEADSET_CAMERA",
        "horizonos.permission.AVATAR_CAMERA"
    )
    foreach ($permission in $permissions) {
        Invoke-Adb -Arguments @("shell", "pm", "grant", $PackageName, $permission) 2>&1 |
            Add-Content -Path (Join-Path $OutDir "permission-grants.txt") -Encoding UTF8
    }
    if ($MediaProjection) {
        "adb shell appops set $PackageName PROJECT_MEDIA allow" |
            Add-Content -Path (Join-Path $OutDir "permission-grants.txt") -Encoding UTF8
        $setOutput = @(Invoke-Adb -Arguments @("shell", "appops", "set", $PackageName, "PROJECT_MEDIA", "allow") 2>&1)
        $setExitCode = $LASTEXITCODE
        $setOutput | Add-Content -Path (Join-Path $OutDir "permission-grants.txt") -Encoding UTF8
        "adb shell appops get $PackageName PROJECT_MEDIA" |
            Add-Content -Path (Join-Path $OutDir "permission-grants.txt") -Encoding UTF8
        $getOutput = @(Invoke-Adb -Arguments @("shell", "appops", "get", $PackageName, "PROJECT_MEDIA") 2>&1)
        $getExitCode = $LASTEXITCODE
        $getOutput | Add-Content -Path (Join-Path $OutDir "permission-grants.txt") -Encoding UTF8
        if ($setExitCode -ne 0 -or $getExitCode -ne 0 -or (($getOutput -join "`n") -notmatch "PROJECT_MEDIA:\s*allow")) {
            throw "MediaProjection PROJECT_MEDIA app-op pregrant failed or did not read back as allow; see permission-grants.txt"
        }
    }
}

function Set-MakepadOculusPerformanceProfile {
    $props = [ordered]@{
        "debug.oculus.cpuLevel" = $OculusCpuLevel
        "debug.oculus.gpuLevel" = $OculusGpuLevel
        "debug.oculus.foveation.level" = $OculusFoveationLevel
        "debug.oculus.foveation.dynamic" = $OculusFoveationDynamic
    }

    foreach ($entry in $props.GetEnumerator()) {
        Set-AdbProperty -Name $entry.Key -Value $entry.Value
    }

    $readback = foreach ($entry in $props.GetEnumerator()) {
        $value = (Invoke-Adb -Arguments @("shell", "getprop", $entry.Key)) -join ""
        [pscustomobject]@{
            property = $entry.Key
            expected = [string]$entry.Value
            actual = $value.Trim()
        }
    }
    $readback | ConvertTo-Json -Depth 3 |
        Set-Content -Path (Join-Path $OutDir "oculus-performance-props.json") -Encoding UTF8
    Assert-PropertyReadback -Readback $readback -Label "oculus-performance"

    return [ordered]@{
        cpuLevel = $OculusCpuLevel
        gpuLevel = $OculusGpuLevel
        foveationLevel = $OculusFoveationLevel
        foveationDynamic = $OculusFoveationDynamic
        readback = $readback
    }
}

function Set-MakepadBrokerH264Profile {
    if ($UseBrokerH264Synthetic -and $UseBrokerH264Camera) {
        throw "Use only one broker H.264 source switch: -UseBrokerH264Synthetic or -UseBrokerH264Camera."
    }

    $brokerRequested = [bool]($UseBrokerH264Synthetic -or $UseBrokerH264Camera)
    $sourceMode = if ($UseBrokerH264Camera) { "broker-camera" } elseif ($UseBrokerH264Synthetic) { "broker-synthetic" } else { "disabled" }
    $projectionGeometryProfile = if ($BrokerH264ProjectionGeometryProfile -and $BrokerH264ProjectionGeometryProfile.Trim().Length -gt 0) {
        $BrokerH264ProjectionGeometryProfile.Trim()
    }
    elseif ($UseBrokerH264Camera) {
        $CameraProjectionGeometryProfile
    }
    elseif ($brokerRequested) {
        $BrokerH264SyntheticProjectionProfile
    }
    else {
        $CameraProjectionGeometryProfile
    }
    $syntheticProjectionProfile = if ($UseBrokerH264Camera -or -not $brokerRequested) {
        $projectionGeometryProfile
    }
    else {
        $BrokerH264SyntheticProjectionProfile
    }
    $props = [ordered]@{
        "debug.rustyquest.makepad.broker.h264.enabled" = if ($brokerRequested) { "true" } else { "false" }
        "debug.rustyquest.makepad.broker.h264.host" = $BrokerH264Host
        "debug.rustyquest.makepad.broker.h264.broker.port" = $BrokerH264BrokerPort
        "debug.rustyquest.makepad.broker.h264.stream.port" = $BrokerH264LeftStreamPort
        "debug.rustyquest.makepad.broker.h264.right.stream.port" = $BrokerH264RightStreamPort
        "debug.rustyquest.makepad.broker.h264.source.mode" = $sourceMode
        "debug.rustyquest.makepad.broker.h264.decode.output.mode" = $BrokerH264DecodeOutputMode
        "debug.rustyquest.makepad.broker.h264.synthetic.pattern" = $BrokerH264SyntheticPattern
        "debug.rustyquest.makepad.broker.h264.projection.geometry.profile" = $projectionGeometryProfile
        "debug.rustyquest.makepad.broker.h264.source.sampling.mode" = $CameraSourceSamplingMode
        "debug.rustyquest.makepad.broker.h264.synthetic.projection.profile" = $syntheticProjectionProfile
        "debug.rustyquest.makepad.broker.h264.target.screen.uv.rect" = $CameraTargetScreenUvRect
        "debug.rustyquest.makepad.broker.h264.left.target.screen.uv.rect" = $CameraLeftTargetScreenUvRect
        "debug.rustyquest.makepad.broker.h264.right.target.screen.uv.rect" = $CameraRightTargetScreenUvRect
        "debug.rustyquest.makepad.broker.h264.left.camera.id" = $BrokerH264LeftCameraId
        "debug.rustyquest.makepad.broker.h264.right.camera.id" = $BrokerH264RightCameraId
        "debug.rustyquest.makepad.broker.h264.width" = $BrokerH264Width
        "debug.rustyquest.makepad.broker.h264.height" = $BrokerH264Height
        "debug.rustyquest.makepad.broker.h264.capture.ms" = $BrokerH264CaptureMs
        "debug.rustyquest.makepad.broker.h264.max.packets" = $BrokerH264MaxPackets
        "debug.rustyquest.makepad.broker.h264.bitrate.bps" = $BrokerH264BitrateBps
        "debug.rustyquest.makepad.broker.h264.frame.rate.hz" = $BrokerH264FrameRateHz
        "debug.rustyquest.makepad.broker.h264.stereo.pair.id" = $BrokerH264StereoPairId
        "debug.rustyquest.makepad.broker.h264.stereo.pair.max.delta.ns" = $BrokerH264StereoPairMaxDeltaNs
        "debug.rustyquest.makepad.broker.h264.stream.timeout.ms" = $BrokerH264StreamTimeoutMs
        "debug.rustyquest.makepad.broker.h264.decode.timeout.ms" = $BrokerH264DecodeTimeoutMs
        "debug.rustyquest.makepad.broker.h264.live.stream" = if ($brokerRequested) { "true" } else { "false" }
    }

    foreach ($entry in $props.GetEnumerator()) {
        Set-AdbProperty -Name $entry.Key -Value $entry.Value
    }

    $readback = foreach ($entry in $props.GetEnumerator()) {
        $value = (Invoke-Adb -Arguments @("shell", "getprop", $entry.Key)) -join ""
        [pscustomobject]@{
            property = $entry.Key
            expected = [string]$entry.Value
            actual = $value.Trim()
        }
    }
    $readback | ConvertTo-Json -Depth 3 |
        Set-Content -Path (Join-Path $OutDir "broker-h264-props.json") -Encoding UTF8

    if (-not $brokerRequested) {
        $readback | ConvertTo-Json -Depth 3 |
            Set-Content -Path (Join-Path $OutDir "broker-h264-disabled-props.json") -Encoding UTF8
        return
    }

    if ($UseBrokerH264Synthetic) {
        $readback | ConvertTo-Json -Depth 3 |
            Set-Content -Path (Join-Path $OutDir "broker-h264-synthetic-props.json") -Encoding UTF8
    }
}

function Set-MakepadProjectionTargetProfile {
    $nativePassthrough = if ($EnableNativePassthrough -or $ProjectionBorderPolicy -eq "passthrough-underlay" -or $ProjectionAreaOpacity -lt 1.0 -or $ProjectionBorderOpacity -lt 1.0 -or $ProjectionAlphaMode -ne "fixed") { "true" } else { "false" }
    $directHardwareBufferExternal = if ($DirectCameraTexturePath -eq "hardware-buffer-external") { "true" } else { "false" }
    # The public suite-level offsets use screenshot/display-screen semantics:
    # positive X moves the projection area right and positive Y moves it down.
    # Makepad's horizontal projection-area properties predate that contract and
    # use the opposite X sign, so normalize X at the wrapper boundary. Native
    # vertical offset already uses the public positive-Y-down contract.
    $offsetLeftUv = if ([double]::IsNaN($ProjectionAreaOffsetLeftUv)) { -$ProjectionAreaOffsetXUv } else { $ProjectionAreaOffsetLeftUv }
    $offsetRightUv = if ([double]::IsNaN($ProjectionAreaOffsetRightUv)) { -$ProjectionAreaOffsetXUv } else { $ProjectionAreaOffsetRightUv }
    $offsetVerticalUv = $ProjectionAreaOffsetYUv
    $canonicalOffsetLeftUv = -$offsetLeftUv
    $canonicalOffsetRightUv = -$offsetRightUv
    $previewFovYDegrees = if ([double]::IsNaN($CameraPreviewFovYDegrees)) { 60.0 } else { $CameraPreviewFovYDegrees }
    $previewOffsetYMeters = if ([double]::IsNaN($CameraPreviewOffsetYMeters)) { 0.0 } else { $CameraPreviewOffsetYMeters }
    $rawOverlayOverscan = if ([double]::IsNaN($CameraRawOverlayOverscan)) { 1.06 } else { $CameraRawOverlayOverscan }
    $props = [ordered]@{
        "debug.rustyquest.makepad.native.passthrough.enabled" = $nativePassthrough
        "debug.rustyquest.makepad.projection.runtime.resolution.enabled" = if ($UseResolvedProjectionRuntime) { "true" } else { "false" }
        "debug.rustyquest.makepad.processing.layer" = $ProcessingLayer
        "debug.rustyquest.makepad.projection.sample.mode" = $ProjectionSampleMode
        "debug.rustyquest.makepad.camera.blur.radius.px" = (Format-InvariantDouble -Value $BlurRadiusPx)
        "debug.rustyquest.makepad.peripheral.stretch.mode" = $PeripheralStretchMode
        "debug.rustyquest.makepad.peripheral.stretch.core.scale" = (Format-InvariantDouble -Value $PeripheralStretchCoreScale)
        "debug.rustyquest.makepad.peripheral.stretch.edge.inset.uv" = (Format-InvariantDouble -Value $PeripheralStretchEdgeInsetUv)
        "debug.rustyquest.makepad.peripheral.stretch.max.inset.uv" = (Format-InvariantDouble -Value $PeripheralStretchMaxInsetUv)
        "debug.rustyquest.makepad.peripheral.stretch.curve" = (Format-InvariantDouble -Value $PeripheralStretchCurve)
        "debug.rustyquest.makepad.peripheral.stretch.inner.blend.uv" = (Format-InvariantDouble -Value $PeripheralStretchInnerBlendUv)
        "debug.rustyquest.makepad.peripheral.stretch.blend.curve" = (Format-InvariantDouble -Value $PeripheralStretchBlendCurve)
        "debug.rustyquest.makepad.peripheral.stretch.blend.mode" = $PeripheralStretchBlendMode
        "debug.rustyquest.makepad.peripheral.stretch.corner.mode" = $PeripheralStretchCornerMode
        "debug.rustyquest.makepad.peripheral.stretch.debug" = $PeripheralStretchDebug
        "debug.rustyquest.makepad.direct.camera.hardware.buffer.external" = $directHardwareBufferExternal
        "debug.rustyquest.makepad.camera.projection.mode" = $CameraProjectionMode
        "debug.rustyquest.makepad.camera.projection.geometry.profile" = $CameraProjectionGeometryProfile
        "debug.rustyquest.makepad.camera.source.sampling.mode" = $CameraSourceSamplingMode
        "debug.rustyquest.makepad.camera.target.screen.uv.rect" = $CameraTargetScreenUvRect
        "debug.rustyquest.makepad.camera.left.target.screen.uv.rect" = $CameraLeftTargetScreenUvRect
        "debug.rustyquest.makepad.camera.right.target.screen.uv.rect" = $CameraRightTargetScreenUvRect
        "debug.rustyquest.makepad.projection.scale" = (Format-InvariantDouble -Value $ProjectionScale)
        "debug.rustyquest.makepad.projection.depth.meters" = (Format-InvariantDouble -Value $ProjectionDepthMeters)
        "debug.rustyquest.makepad.camera.preview.fov.y.degrees" = (Format-InvariantDouble -Value $previewFovYDegrees)
        "debug.rustyquest.makepad.camera.preview.offset.y.meters" = (Format-InvariantDouble -Value $previewOffsetYMeters)
        "debug.rustyquest.makepad.camera.raw.overlay.overscan" = (Format-InvariantDouble -Value $rawOverlayOverscan)
        "debug.rustyquest.makepad.xr.render.scale" = (Format-InvariantDouble -Value $XrRenderScale)
        "debug.rustyquest.makepad.xr.display.refresh.rate.hz" = (Format-InvariantDouble -Value $XrDisplayRefreshHz)
        "debug.rustyquest.makepad.projection.area.left.offset.x.uv" = (Format-InvariantDouble -Value $canonicalOffsetLeftUv)
        "debug.rustyquest.makepad.projection.area.right.offset.x.uv" = (Format-InvariantDouble -Value $canonicalOffsetRightUv)
        "debug.rustyquest.makepad.projection.area.offset.y.uv" = (Format-InvariantDouble -Value $offsetVerticalUv)
        "debug.rustyquest.makepad.projection.area.scale.x" = (Format-InvariantDouble -Value $ProjectionAreaScaleX)
        "debug.rustyquest.makepad.projection.area.scale.y" = (Format-InvariantDouble -Value $ProjectionAreaScaleY)
        "debug.rustyquest.makepad.projection.target.offset.x.uv" = (Format-InvariantDouble -Value $ProjectionTargetOffsetXUv)
        "debug.rustyquest.makepad.projection.target.offset.y.uv" = (Format-InvariantDouble -Value $ProjectionTargetOffsetYUv)
        "debug.rustyquest.makepad.projection.target.scale" = (Format-InvariantDouble -Value $ProjectionTargetScale)
        "debug.rustyquest.makepad.projection.target.joystick.controls" = $ProjectionTargetJoystickControls
        "debug.rustyquest.makepad.projection.area.radius.x.uv" = (Format-InvariantDouble -Value $ProjectionAreaRadiusXUv)
        "debug.rustyquest.makepad.projection.area.radius.y.uv" = (Format-InvariantDouble -Value $ProjectionAreaRadiusYUv)
        "debug.rustyquest.makepad.projection.area.corner.radius.uv" = (Format-InvariantDouble -Value $ProjectionAreaCornerRadiusUv)
        "debug.rustyquest.makepad.projection.area.opacity" = (Format-InvariantDouble -Value $ProjectionAreaOpacity)
        "debug.rustyquest.makepad.projection.border.opacity" = (Format-InvariantDouble -Value $ProjectionBorderOpacity)
        "debug.rustyquest.makepad.projection.area.diagnostic" = (Format-InvariantDouble -Value $ProjectionAreaDiagnostic)
        "debug.rustyquest.makepad.projection.border.policy" = $ProjectionBorderPolicy
        "debug.rustyquest.makepad.projection.alpha.mode" = $ProjectionAlphaMode
        "debug.rustyquest.makepad.projection.alpha.scale" = (Format-InvariantDouble -Value $ProjectionAlphaScale)
        "debug.rustyquest.makepad.projection.alpha.bias" = (Format-InvariantDouble -Value $ProjectionAlphaBias)
    }

    foreach ($entry in $props.GetEnumerator()) {
        Set-AdbProperty -Name $entry.Key -Value $entry.Value
    }

    $readback = foreach ($entry in $props.GetEnumerator()) {
        $value = (Invoke-Adb -Arguments @("shell", "getprop", $entry.Key)) -join ""
        [pscustomobject]@{
            property = $entry.Key
            expected = [string]$entry.Value
            actual = $value.Trim()
        }
    }
    $readback | ConvertTo-Json -Depth 3 |
        Set-Content -Path (Join-Path $OutDir "projection-target-props.json") -Encoding UTF8
    Assert-PropertyReadback -Readback $readback -Label "projection-target"
}

function Install-Apk {
    if ($SkipInstall) {
        return
    }
    if (-not (Test-Path -LiteralPath $Apk)) {
        throw "APK not found: $Apk"
    }

    Invoke-Adb -Arguments @("shell", "am", "force-stop", $PackageName) | Out-Null
    Invoke-Adb -Arguments @("uninstall", $PackageName) 2>&1 |
        Set-Content -Path (Join-Path $OutDir "uninstall.txt") -Encoding UTF8

    $installPath = Join-Path $OutDir "install.txt"
    $installOutput = Invoke-Adb -Arguments @("install", "--no-incremental", "-r", "-d", $Apk) 2>&1
    $installOutput | Set-Content -Path $installPath -Encoding UTF8
    if ($LASTEXITCODE -ne 0 -or (($installOutput -join "`n") -notmatch "Success")) {
        $fallbackOutput = Invoke-Adb -Arguments @("install", "-r", "-d", $Apk) 2>&1
        $fallbackOutput | Add-Content -Path $installPath -Encoding UTF8
        if ($LASTEXITCODE -ne 0 -or (($fallbackOutput -join "`n") -notmatch "Success")) {
            throw "adb install failed; see $installPath"
        }
    }
}

function Stop-PreLaunchPackages {
    Invoke-RustyXrPublicExampleSiblingForceStop `
        -Adb "adb" `
        -Serial $Serial `
        -ActivePackageName $PackageName `
        -PackageNames $PreLaunchForceStopPackages `
        -OutputPath (Join-Path $OutDir "prelaunch-force-stop-packages.json") `
        -Skip:$SkipPreLaunchForceStopPackages
}

function Capture-LaunchState {
    param(
        [string]$Label,
        [datetime]$LaunchStartedAt
    )
    $capturedAt = Get-Date
    $dir = Join-Path $OutDir $Label
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    Save-Adb -Arguments @("shell", "dumpsys", "activity", "activities") -Path (Join-Path $dir "activity.txt")
    Save-Adb -Arguments @("shell", "dumpsys", "window", "windows") -Path (Join-Path $dir "window.txt")
    Save-Adb -Arguments @("logcat", "-d", "-v", "threadtime") -Path (Join-Path $dir "logcat.txt")

    $log = Get-Content -Path (Join-Path $dir "logcat.txt")
    $activity = Get-Content -Path (Join-Path $dir "activity.txt")
    $window = Get-Content -Path (Join-Path $dir "window.txt")
    $processId = ((Invoke-Adb -Arguments @("shell", "pidof", $PackageName) 2>$null) -join " ").Trim()
    $appLog = @()
    if ($processId) {
        $pidPattern = "\s$([regex]::Escape($processId))\s"
        $appLog = @($log | Select-String -Pattern $pidPattern | ForEach-Object { $_.Line })
    }
    $appPattern = [regex]::Escape($PackageName)
    $xrActivityPattern = [regex]::Escape($XrActivity.TrimStart("."))
    $activityHasExpectedPackage = @($activity | Select-String -Pattern $appPattern).Count -gt 0
    $windowHasExpectedPackage = @($window | Select-String -Pattern $appPattern).Count -gt 0
    $activityHasExpectedXrActivity = @($activity | Select-String -Pattern $xrActivityPattern).Count -gt 0
    $windowHasExpectedXrActivity = @($window | Select-String -Pattern $xrActivityPattern).Count -gt 0
    $activeXr = $activityHasExpectedPackage -and
        $windowHasExpectedPackage -and
        $activityHasExpectedXrActivity -and
        $windowHasExpectedXrActivity
    $openxrEndFrame = @($log | Select-String -SimpleMatch "RUSTY_QUEST_MAKEPAD_OPENXR_END_FRAME").Count
    $frameFlowEndFrame = @($log | Select-String -Pattern "RUSTY_QUEST_MAKEPAD_FRAME_FLOW.*phase=xr-end-frame.*status=submitted").Count
    $endFrame = $openxrEndFrame + $frameFlowEndFrame
    $frameAdoptionLines = @($log | Select-String -SimpleMatch "RUSTY_QUEST_MAKEPAD_FRAME_ADOPTION" | ForEach-Object { $_.Line })
    $frameAdoptionCount = @($frameAdoptionLines).Count
    $frameAdoptionPoseMatchedCount = @($frameAdoptionLines | Select-String -SimpleMatch "poseSource=makepad-quest-update-predicted-display-pose").Count
    $frameAdoptionCloseTimestampMatchCount = @($frameAdoptionLines | Select-String -SimpleMatch "closeTimestampMatch=true").Count
    $frameAdoptionTimingGapCount = @($frameAdoptionLines | Select-String -SimpleMatch "pairingStatus=latest-complete-with-timing-gap").Count
    $latestFrameAdoptionLine = @($frameAdoptionLines | Select-Object -Last 1)
    $visiblePanel = @($log | Select-String -SimpleMatch "visibleCameraProjectionReady=true").Count
    $xrCadence = @($log | Select-String -Pattern "RUSTY_QUEST_MAKEPAD_CADENCE.*xrUpdateRateHz=(?!0\\.00)").Count
    $loadingSignals = @($log | Select-String -Pattern "(?i)XrPermissionsFlow|preflight|loading").Count
    $brokerH264PrepareRequestCount = @($log | Select-String -SimpleMatch "phase=broker-h264-prepare-request status=sent").Count
    $brokerH264UnboundedHeaderCount = @($log | Select-String -SimpleMatch "packets=0 metadataBytes=").Count
    $brokerH264StreamHeaderMetadataCount = @($log | Select-String -SimpleMatch "phase=stream-header-metadata status=ok").Count
    $leftCameraIdPattern = [regex]::Escape($BrokerH264LeftCameraId.Trim())
    $rightCameraIdPattern = [regex]::Escape($BrokerH264RightCameraId.Trim())
    $brokerH264LeftRequestedCameraHeaderCount = if ($leftCameraIdPattern.Length -gt 0) {
        @($log | Select-String -Pattern "phase=stream-header-metadata status=ok side=left .*cameraId=$leftCameraIdPattern(?:\s|$)").Count
    } else { 0 }
    $brokerH264RightRequestedCameraHeaderCount = if ($rightCameraIdPattern.Length -gt 0) {
        @($log | Select-String -Pattern "phase=stream-header-metadata status=ok side=right .*cameraId=$rightCameraIdPattern(?:\s|$)").Count
    } else { 0 }
    $brokerH264PreparedCount = @($log | Select-String -SimpleMatch "phase=prepared status=ok").Count
    $brokerH264YuvTexturesReadyCount = @($log | Select-String -SimpleMatch "textureMode=cpu-yuv-decoded-broker-h264").Count
    $brokerH264YuvTextureUpdateCount = @($log | Select-String -Pattern "phase=texture-updated status=ok.*cpuUploadPath=broker-h264-mediacodec-cpu-yuv").Count
    $brokerH264HardwareBufferTextureUpdateCount = @($log | Select-String -Pattern "phase=texture-updated status=ok.*cameraTexturePath=broker-h264-mediacodec-hardware-buffer").Count
    $brokerH264HardwareBufferFrameCount = @($log | Select-String -SimpleMatch "RUSTY_QUEST_MAKEPAD_BROKER_H264_HARDWARE_BUFFER_FRAME").Count
    $brokerH264TextureUpdateCount = $brokerH264YuvTextureUpdateCount + $brokerH264HardwareBufferTextureUpdateCount
    $brokerH264DecodeErrorCount = @($log | Select-String -Pattern "event=decode-error|Broker H[.]264 playback failed").Count
    $brokerH264ProgressLines = @($log | Select-String -SimpleMatch "Broker H.264 playback progress" | ForEach-Object { $_.Line })
    $brokerH264ProgressCount = @($brokerH264ProgressLines).Count
    $brokerH264CompleteProgressCount = @($brokerH264ProgressLines | Select-String -SimpleMatch "phase=complete").Count
    $brokerH264PacketsReadMax = 0
    $brokerH264InputQueuedMax = 0
    $brokerH264DecodedFrameMax = 0
    $brokerH264YuvFrameEmitMax = 0
    $brokerH264HardwareBufferFrameEmitMax = 0
    $brokerH264PacketReadRateHzMax = 0.0
    $brokerH264InputQueueRateHzMax = 0.0
    $brokerH264DecodedFrameRateHzMax = 0.0
    $brokerH264YuvFrameEmitRateHzMax = 0.0
    $brokerH264HardwareBufferFrameEmitRateHzMax = 0.0
    $brokerH264YuvCopyTimeMsMax = 0
    $brokerH264YuvCopyAvgMsMax = 0.0
    foreach ($line in $brokerH264ProgressLines) {
        if ($line -match "packetsRead=(\d+)") {
            $brokerH264PacketsReadMax = [Math]::Max($brokerH264PacketsReadMax, [int]$Matches[1])
        }
        if ($line -match "inputQueuedCount=(\d+)") {
            $brokerH264InputQueuedMax = [Math]::Max($brokerH264InputQueuedMax, [int]$Matches[1])
        }
        if ($line -match "decodedFrameCount=(\d+)") {
            $brokerH264DecodedFrameMax = [Math]::Max($brokerH264DecodedFrameMax, [int]$Matches[1])
        }
        if ($line -match "yuvFrameEmitCount=(\d+)") {
            $brokerH264YuvFrameEmitMax = [Math]::Max($brokerH264YuvFrameEmitMax, [int]$Matches[1])
        }
        if ($line -match "hardwareBufferFrameEmitCount=(\d+)") {
            $brokerH264HardwareBufferFrameEmitMax = [Math]::Max($brokerH264HardwareBufferFrameEmitMax, [int]$Matches[1])
        }
        if ($line -match "yuvCopyTimeMs=(\d+)") {
            $brokerH264YuvCopyTimeMsMax = [Math]::Max($brokerH264YuvCopyTimeMsMax, [int]$Matches[1])
        }
        if ($line -match "yuvCopyAvgMs=([0-9.]+)") {
            $brokerH264YuvCopyAvgMsMax = [Math]::Max($brokerH264YuvCopyAvgMsMax, (Parse-DoubleInvariant $Matches[1]))
        }
        if ($line -match "packetReadRateHz=([0-9.]+)") {
            $brokerH264PacketReadRateHzMax = [Math]::Max($brokerH264PacketReadRateHzMax, (Parse-DoubleInvariant $Matches[1]))
        }
        if ($line -match "inputQueueRateHz=([0-9.]+)") {
            $brokerH264InputQueueRateHzMax = [Math]::Max($brokerH264InputQueueRateHzMax, (Parse-DoubleInvariant $Matches[1]))
        }
        if ($line -match "decodedFrameRateHz=([0-9.]+)") {
            $brokerH264DecodedFrameRateHzMax = [Math]::Max($brokerH264DecodedFrameRateHzMax, (Parse-DoubleInvariant $Matches[1]))
        }
        if ($line -match "yuvFrameEmitRateHz=([0-9.]+)") {
            $brokerH264YuvFrameEmitRateHzMax = [Math]::Max($brokerH264YuvFrameEmitRateHzMax, (Parse-DoubleInvariant $Matches[1]))
        }
        if ($line -match "hardwareBufferFrameEmitRateHz=([0-9.]+)") {
            $brokerH264HardwareBufferFrameEmitRateHzMax = [Math]::Max($brokerH264HardwareBufferFrameEmitRateHzMax, (Parse-DoubleInvariant $Matches[1]))
        }
    }
    $pairedCameraFrameCadenceCount = @($log | Select-String -SimpleMatch "pairedLeftRightCameraFrames=true").Count
    $alignedProjectionCadenceCount = @($log | Select-String -SimpleMatch "alignedProjection=true").Count
    $projectionMappingReadyCadenceCount = @($log | Select-String -SimpleMatch "projectionMappingReady=true").Count
    $leftTextureUpdateMax = 0
    $rightTextureUpdateMax = 0
    foreach ($line in @($log | Select-String -SimpleMatch "RUSTY_QUEST_MAKEPAD_CADENCE" | ForEach-Object { $_.Line })) {
        if ($line -match "leftTextureUpdateCount=(\d+)") {
            $leftTextureUpdateMax = [Math]::Max($leftTextureUpdateMax, [int]$Matches[1])
        }
        if ($line -match "rightTextureUpdateCount=(\d+)") {
            $rightTextureUpdateMax = [Math]::Max($rightTextureUpdateMax, [int]$Matches[1])
        }
    }
    $projectionRuntimeManifestLines = @($log | Select-String -SimpleMatch "RUSTY_QUEST_MAKEPAD_PROJECTION_RUNTIME_MANIFEST" | ForEach-Object { $_.Line })
    $projectionRuntimeNumericTypeIssues = @(Get-ProjectionRuntimeNumericTypeIssues -LogLines $projectionRuntimeManifestLines)
    $broadGpuFaultSignalPattern = "(?i)page fault|gpu.*fault|kgsl|iommu|CP_SQE|faulting"
    $kernelGpuFaultPattern = "(?i)GPU PAGE FAULT|premature free|already freed|kgsl_iommu_print_fault|kgsl_mmu_pagefault_resume|CP_SQE|(?:read|write) translation fault"
    $appKernelGpuFaultCount = @($appLog | Select-String -Pattern $kernelGpuFaultPattern).Count
    $kernelGpuFaultCount = @($log | Select-String -Pattern $kernelGpuFaultPattern).Count

    $state = [ordered]@{
        label = $Label
        launchedAt = $LaunchStartedAt.ToString("o")
        capturedAt = $capturedAt.ToString("o")
        elapsedSinceLaunchMs = [long]($capturedAt - $LaunchStartedAt).TotalMilliseconds
        activeXrActivity = [bool]$activeXr
        activityHasExpectedPackage = [bool]$activityHasExpectedPackage
        windowHasExpectedPackage = [bool]$windowHasExpectedPackage
        activityHasExpectedXrActivity = [bool]$activityHasExpectedXrActivity
        windowHasExpectedXrActivity = [bool]$windowHasExpectedXrActivity
        openxrEndFrameCount = $endFrame
        openxrRuntimeEndFrameMarkerCount = $openxrEndFrame
        frameFlowEndFrameMarkerCount = $frameFlowEndFrame
        frameAdoptionMarkerCount = $frameAdoptionCount
        frameAdoptionPoseMatchedMarkerCount = $frameAdoptionPoseMatchedCount
        frameAdoptionCloseTimestampMatchMarkerCount = $frameAdoptionCloseTimestampMatchCount
        frameAdoptionTimingGapMarkerCount = $frameAdoptionTimingGapCount
        latestFrameAdoptionMarker = if ($latestFrameAdoptionLine) { [string]$latestFrameAdoptionLine } else { "" }
        visiblePanelMarkerCount = $visiblePanel
        nonzeroXrCadenceMarkerCount = $xrCadence
        loadingSignalCount = $loadingSignals
        processId = $processId
        appLineCount = @($appLog).Count
        appGpuFaultCount = $appKernelGpuFaultCount
        gpuFaultCount = $kernelGpuFaultCount
        appKernelGpuFaultCount = $appKernelGpuFaultCount
        kernelGpuFaultCount = $kernelGpuFaultCount
        kernelGpuPageFaultCount = @($log | Select-String -SimpleMatch "GPU PAGE FAULT").Count
        kernelGpuPrematureFreeCount = @($log | Select-String -SimpleMatch "premature free").Count
        kernelGpuAlreadyFreedCount = @($log | Select-String -SimpleMatch "already freed").Count
        appBroadGpuFaultSignalCount = @($appLog | Select-String -Pattern $broadGpuFaultSignalPattern).Count
        broadGpuFaultSignalCount = @($log | Select-String -Pattern $broadGpuFaultSignalPattern).Count
        fatalCount = @($log | Select-String -Pattern "FATAL EXCEPTION|Fatal signal|signal 11|SIGSEGV|Abort message").Count
        hardwareBufferWarningCount = @($log | Select-String -Pattern "(?i)hardware.?buffer|AHardwareBuffer|GraphicBuffer\(w=4").Count
        projectionRuntimeManifestCount = $projectionRuntimeManifestLines.Count
        resolvedProjectionRuntimeEnabledMarkerCount = @($log | Select-String -SimpleMatch "RUSTY_QUEST_MAKEPAD_PROJECTION_RUNTIME" | Select-String -SimpleMatch "resolvedManifestConsumptionEnabled=true").Count
        projectionRuntimeNumericTypeIssueCount = $projectionRuntimeNumericTypeIssues.Count
        projectionRuntimeNumericTypeIssues = $projectionRuntimeNumericTypeIssues
        s69bMarkerCount = @($log | Select-String -SimpleMatch "s69bHorizontalMirrorFix=true").Count
        s70SquareAspectMarkerCount = @($log | Select-String -SimpleMatch "s70SquareAspectFix=true").Count
        s72HeadCenteredSquareRestoredMarkerCount = @($log | Select-String -SimpleMatch "s72HeadCenteredSquareRestored=true").Count
        s72MetadataUvBaselineCorrectionMarkerCount = @($log | Select-String -SimpleMatch "s72MetadataUvBaselineCorrection=true").Count
        s73ScalarHomographyBindingMarkerCount = @($log | Select-String -SimpleMatch "s73ScalarHomographyBinding=true").Count
        s74LiteralHomographyRowsMarkerCount = @($log | Select-String -SimpleMatch "s74LiteralHomographyRows=true").Count
        s75DynamicHomographyBindingMarkerCount = @($log | Select-String -SimpleMatch "s75DynamicHomographyBinding=true").Count
        s76DirectDrawVarsHomographyMarkerCount = @($log | Select-String -SimpleMatch "s76DirectDrawVarsHomography=true").Count
        s77SourceUvValidityFallbackMarkerCount = @($log | Select-String -SimpleMatch "s77SourceUvValidityFallback=true").Count
        s78ClipSpaceSurfaceHomographyMarkerCount = @($log | Select-String -SimpleMatch "s78ClipSpaceSurfaceHomography=true").Count
        s79TargetSourceEyeMappingMarkerCount = @($log | Select-String -SimpleMatch "s79TargetSourceEyeMapping=true").Count
        s80FullViewContentUvScaleMarkerCount = @($log | Select-String -SimpleMatch "s80FullViewContentUvScale=true").Count
        s81DynamicScreenSurfaceUvMarkerCount = @($log | Select-String -SimpleMatch "s81DynamicScreenSurfaceUv=true").Count
        s82CollapsedScreenToCameraHomographyMarkerCount = @($log | Select-String -SimpleMatch "s82CollapsedScreenToCameraHomography=true").Count
        s83DrawPassProjectionInverseHomographyMarkerCount = @($log | Select-String -SimpleMatch "s83DrawPassProjectionInverseHomography=true").Count
        s84ProjectionInverseNearFarFallbackMarkerCount = @($log | Select-String -SimpleMatch "s84ProjectionInverseNearFarFallback=true").Count
        s85ForcedScreenToCameraFallbackMarkerCount = @($log | Select-String -SimpleMatch "s85ForcedScreenToCameraFallback=true").Count
        s87RuntimeXrViewHomographyMarkerCount = @($log | Select-String -SimpleMatch "s87RuntimeXrViewHomography=true").Count
        s88SourceValidityFallbackMarkerCount = @($log | Select-String -SimpleMatch "s88SourceValidityFallback=true").Count
        s89SingleQuadTargetScreenUvMarkerCount = @($log | Select-String -SimpleMatch "s89SingleQuadTargetScreenUv=true").Count
        s90CameraIdSourceBindingMarkerCount = @($log | Select-String -SimpleMatch "s90CameraIdSourceBinding=true").Count
        s91ProjectionMathCorrectionMarkerCount = @($log | Select-String -SimpleMatch "s91ProjectionMathCorrection=true").Count
        s91InvertedSourceEyeSelectorMarkerCount = @($log | Select-String -SimpleMatch "s91InvertedSourceEyeSelector=true").Count
        s91DisplayIndexedHomographyRowsMarkerCount = @($log | Select-String -SimpleMatch "s91DisplayIndexedHomographyRows=true").Count
        s91VerticalOnlyTextureUvMarkerCount = @($log | Select-String -SimpleMatch "s91VerticalOnlyTextureUv=true").Count
        s98NativePassthroughHudSplitMarkerCount = @($log | Select-String -SimpleMatch "s98NativePassthroughHudSplit=true").Count
        s100DefaultRenderScaleHudControlMarkerCount = @($log | Select-String -SimpleMatch "s100DefaultRenderScaleHudControl=true").Count
        s101CameraFeedSuppressedHudControlMarkerCount = @($log | Select-String -SimpleMatch "s101CameraFeedSuppressedHudControl=true").Count
        liveCameraSamplingSuppressedMarkerCount = @($log | Select-String -SimpleMatch "liveCameraSamplingSuppressed=true").Count
        s102FullSurfaceLiveCameraCoverageControlMarkerCount = @($log | Select-String -SimpleMatch "s102FullSurfaceLiveCameraCoverageControl=true").Count
        forceFullSurfaceLiveCameraUvMarkerCount = @($log | Select-String -SimpleMatch "forceFullSurfaceLiveCameraUv=true").Count
        s103InSurfaceCameraWindowBorderControlMarkerCount = @($log | Select-String -SimpleMatch "s103InSurfaceCameraWindowBorderControl=true").Count
        s104HorizontalWindowAlignmentControlMarkerCount = @($log | Select-String -SimpleMatch "s104HorizontalWindowAlignmentControl=true").Count
        horizontalAlignmentCenterDeltaMarkerCount = @($log | Select-String -SimpleMatch "horizontalAlignmentSource=surface_to_camera_center_delta").Count
        s105HotloadHorizontalAlignmentControlMarkerCount = @($log | Select-String -SimpleMatch "s105HotloadHorizontalAlignmentControl=true").Count
        s106SafeHorizontalWindowSamplingMarkerCount = @($log | Select-String -SimpleMatch "s106SafeHorizontalWindowSampling=true").Count
        s107WindowScaleHotloadMarkerCount = @($log | Select-String -SimpleMatch "s107WindowScaleHotload=true").Count
        s108BorderlessWindowScaleMarkerCount = @($log | Select-String -SimpleMatch "s108BorderlessWindowScale=true").Count
        horizontalAlignmentScreenCenterDeltaMarkerCount = @($log | Select-String -SimpleMatch "horizontalAlignmentSource=screen_to_camera_center_delta").Count
        horizontalAlignmentSafeWindowMarkerCount = @($log | Select-String -SimpleMatch "horizontalAlignmentSource=screen_to_camera_center_delta_projection_area_source_valid_window").Count
        manualHorizontalOffsetHotloadMarkerCount = @($log | Select-String -SimpleMatch "manualHorizontalOffsetHotload=true").Count
        contentUvScaleHotloadMarkerCount = @($log | Select-String -SimpleMatch "contentUvScaleHotload=true").Count
        borderlessWindowMaskMarkerCount = @($log | Select-String -SimpleMatch "borderlessWindowMask=true").Count
        forceInSurfaceCameraWindowMarkerCount = @($log | Select-String -SimpleMatch "forceInSurfaceCameraWindow=true").Count
        fullSurfaceLayerActiveMarkerCount = @($log | Select-String -SimpleMatch "fullSurfaceLayerActive=true").Count
        cameraCoverageInShaderMarkerCount = @($log | Select-String -SimpleMatch "cameraCoverageInShader=true").Count
        layerNotResizedMarkerCount = @($log | Select-String -SimpleMatch "layerNotResized=true").Count
        projectionValidMaskDisabledMarkerCount = @($log | Select-String -SimpleMatch "projectionValidMaskDisabled=true").Count
        highDefaultImageRectMarkerCount = @($log | Select-String -SimpleMatch "imageRectWidth=2352 imageRectHeight=2464").Count
        reducedImageRectMarkerCount = @($log | Select-String -SimpleMatch "imageRectWidth=1260 imageRectHeight=1320").Count
        cameraIdSourceBindingMarkerCount = @($log | Select-String -SimpleMatch "sourceBindingMode=camera-id").Count
        s86DirectYuvFullscreenControlMarkerCount = @($log | Select-String -SimpleMatch "s86DirectYuvFullscreenControl=true").Count
        runtimeXrViewStateReadyMarkerCount = @($log | Select-String -SimpleMatch "runtimeXrViewStateReady=true").Count
        staleS81PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s81-dynamic-screen-surface-panel-control").Count
        staleS82PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s82-collapsed-screen-to-camera-homography-panel-control").Count
        staleS83PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s83-draw-pass-projection-inverse-homography-panel-control").Count
        staleS84PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s84-projection-inverse-near-far-s82-fallback-panel-control").Count
        staleS86PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s86-direct-yuv-fullscreen-control").Count
        staleS85PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s85-forced-screen-to-camera-fallback-control").Count
        staleS87PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s87-runtime-xr-view-homography").Count
        staleS88PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s88-source-validity-fallback").Count
        staleS90PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s90-camera-id-bound-single-quad-target-screen-uv").Count
        brokerH264StartupMarkerCount = @($log | Select-String -SimpleMatch "status=broker-h264-enabled").Count
        brokerH264ImportPlanMarkerCount = @($log | Select-String -SimpleMatch "importPlan=broker-h264-stereo-mediacodec-yuv-texture").Count
        brokerH264PrepareRequestMarkerCount = $brokerH264PrepareRequestCount
        brokerH264UnboundedStreamHeaderMarkerCount = $brokerH264UnboundedHeaderCount
        brokerH264StreamHeaderMetadataMarkerCount = $brokerH264StreamHeaderMetadataCount
        brokerH264LeftRequestedCameraHeaderMarkerCount = $brokerH264LeftRequestedCameraHeaderCount
        brokerH264RightRequestedCameraHeaderMarkerCount = $brokerH264RightRequestedCameraHeaderCount
        brokerH264PreparedMarkerCount = $brokerH264PreparedCount
        brokerH264YuvTexturesReadyMarkerCount = $brokerH264YuvTexturesReadyCount
        brokerH264TextureUpdateMarkerCount = $brokerH264TextureUpdateCount
        brokerH264YuvTextureUpdateMarkerCount = $brokerH264YuvTextureUpdateCount
        brokerH264HardwareBufferTextureUpdateMarkerCount = $brokerH264HardwareBufferTextureUpdateCount
        brokerH264HardwareBufferFrameMarkerCount = $brokerH264HardwareBufferFrameCount
        brokerH264DecodeErrorMarkerCount = $brokerH264DecodeErrorCount
        brokerH264ProgressMarkerCount = $brokerH264ProgressCount
        brokerH264CompleteProgressMarkerCount = $brokerH264CompleteProgressCount
        brokerH264PacketsReadMax = $brokerH264PacketsReadMax
        brokerH264InputQueuedMax = $brokerH264InputQueuedMax
        brokerH264DecodedFrameMax = $brokerH264DecodedFrameMax
        brokerH264YuvFrameEmitMax = $brokerH264YuvFrameEmitMax
        brokerH264HardwareBufferFrameEmitMax = $brokerH264HardwareBufferFrameEmitMax
        brokerH264YuvCopyTimeMsMax = $brokerH264YuvCopyTimeMsMax
        brokerH264YuvCopyAvgMsMax = $brokerH264YuvCopyAvgMsMax
        brokerH264PacketReadRateHzMax = $brokerH264PacketReadRateHzMax
        brokerH264InputQueueRateHzMax = $brokerH264InputQueueRateHzMax
        brokerH264DecodedFrameRateHzMax = $brokerH264DecodedFrameRateHzMax
        brokerH264YuvFrameEmitRateHzMax = $brokerH264YuvFrameEmitRateHzMax
        brokerH264HardwareBufferFrameEmitRateHzMax = $brokerH264HardwareBufferFrameEmitRateHzMax
        brokerH264DecodedTextureReady = [bool]($brokerH264PreparedCount -gt 0 -and $brokerH264TextureUpdateCount -gt 0 -and $brokerH264DecodeErrorCount -eq 0)
        brokerH264LeftTextureUpdateMax = $leftTextureUpdateMax
        brokerH264RightTextureUpdateMax = $rightTextureUpdateMax
        pairedCameraFrameCadenceMarkerCount = $pairedCameraFrameCadenceCount
        alignedProjectionCadenceMarkerCount = $alignedProjectionCadenceCount
        projectionMappingReadyCadenceMarkerCount = $projectionMappingReadyCadenceCount
        brokerH264StereoProofMarkerCount = @($log | Select-String -Pattern "cpuUploadPath=broker-h264-mediacodec-cpu-yuv|cameraTexturePath=broker-h264-mediacodec-hardware-buffer").Count
        brokerH264SyntheticSourceMarkerCount = @($log | Select-String -SimpleMatch "sourceBindingMode=broker-h264-synthetic-stereo-stream").Count
        projectionHomographyReadyMarkerCount = @($log | Select-String -SimpleMatch "projectionHomographyReady=true").Count
        s71EyeCenteredMarkerCount = @($log | Select-String -SimpleMatch "s71EyeCenteredPanel=true").Count
        s71SharedPlaneParallaxRemovedMarkerCount = @($log | Select-String -SimpleMatch "s71SharedPlaneParallaxRemoved=true").Count
        staleS80PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s80-target-full-view-content-scale-panel-control").Count
        staleS79PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s79-target-source-eye-mapping-panel-control").Count
        staleS78PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s78-clipspace-surface-homography-panel-control").Count
        staleS77PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s77-rusty-quest-invalid-uv-fallback-panel-control").Count
        staleS76PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s76-direct-drawvars-homography-panel-control").Count
        staleS75PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s75-dynamic-homography-panel-control").Count
        staleS71PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s71-eye-centered-square-panel-control").Count
        staleS70PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s70-head-centered-aspect-panel-control").Count
        staleS69PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s69-source-eye-swap-panel-control").Count
        staleS68PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s68-active-eye-nonworld-panel-control").Count
        ready = [bool]($activeXr -and $processId -and $endFrame -gt 0 -and ($visiblePanel -gt 0 -or $xrCadence -gt 0))
        readySignals = [ordered]@{
            activeXrActivity = [bool]$activeXr
            processIdPresent = -not [string]::IsNullOrWhiteSpace($processId)
            openxrEndFrameCount = $endFrame
            visiblePanelMarkerCount = $visiblePanel
            nonzeroXrCadenceMarkerCount = $xrCadence
        }
        resumed = @($activity | Select-String -Pattern "ResumedActivity|topResumedActivity|$xrActivityPattern|$appPattern" | ForEach-Object { $_.Line.Trim() })
        focus = @($window | Select-String -Pattern "mCurrentFocus|mFocusedApp|mResumedActivity" | ForEach-Object { $_.Line.Trim() })
    }
    $state | ConvertTo-Json -Depth 5 | Set-Content -Path (Join-Path $dir "state.json") -Encoding UTF8
    return $state
}

function Start-ActivityAndProbe {
    param(
        [string]$Label,
        [string]$Activity,
        [switch]$ForceStopFirst,
        [switch]$LauncherIntent,
        [switch]$VrIntent
    )
    if ($LauncherIntent -and $VrIntent) {
        throw "LauncherIntent and VrIntent are mutually exclusive."
    }
    if ($ForceStopFirst) {
        Invoke-Adb -Arguments @("shell", "am", "force-stop", $PackageName) | Out-Null
    }
    Invoke-Adb -Arguments @("logcat", "-c") | Out-Null
    $component = Activity-Component -Activity $Activity
    $launchStartedAt = Get-Date
    $launchArgs = @("shell", "am", "start", "-W")
    if ($LauncherIntent) {
        $launchArgs += @("-a", "android.intent.action.MAIN", "-c", "android.intent.category.LAUNCHER")
    }
    elseif ($VrIntent) {
        $launchArgs += @("-a", "android.intent.action.MAIN", "-c", "com.oculus.intent.category.VR")
    }
    $launchArgs += @("-n", $component)
    if ($MediaProjection) {
        $launchArgs += @(
            "--ez", "rustyquest.makepad.mediaProjection", "true",
            "--ei", "rustyquest.makepad.mediaProjectionPort", $MediaProjectionPort.ToString(),
            "--ei", "rustyquest.makepad.mediaProjectionWidth", $MediaProjectionWidth.ToString(),
            "--ei", "rustyquest.makepad.mediaProjectionHeight", $MediaProjectionHeight.ToString(),
            "--ei", "rustyquest.makepad.mediaProjectionDelayMs", $MediaProjectionDelayMs.ToString()
        )
    }
    Save-Adb -Arguments $launchArgs -Path (Join-Path $OutDir "$Label-start.txt") -TimeoutSeconds $StartupTimeoutSeconds

    $deadline = (Get-Date).AddSeconds($StartupTimeoutSeconds)
    do {
        Start-Sleep -Milliseconds $ReadyPollIntervalMs
        $state = Capture-LaunchState -Label $Label -LaunchStartedAt $launchStartedAt
        if ($state.ready) {
            return $state
        }
    } while ((Get-Date) -lt $deadline)
    return $state
}

function Capture-FreshnessFrames {
    param([string]$Label)
    $dir = Join-Path $OutDir $Label
    $shotDir = Join-Path $dir "screenshots"
    New-Item -ItemType Directory -Force -Path $shotDir | Out-Null
    $hashes = @()
    for ($i = 0; $i -lt $FreshnessFrames; $i++) {
        $remote = "/sdcard/rustyquest_makepad_${Label}_$i.png"
        $local = Join-Path $shotDir ("{0}-frame-{1:D2}.png" -f $Label, $i)
        Invoke-Adb -Arguments @("shell", "screencap", "-p", $remote) | Out-Null
        Receive-AdbFile -Remote $remote -Local $local
        Invoke-Adb -Arguments @("shell", "rm", $remote) | Out-Null
        $localLong = Convert-ToLongLiteralPath -Path $local
        $hashes += [ordered]@{
            file = $local
            sha256 = Get-Sha256Hex -Path $localLong
            length = (Get-Item -LiteralPath $localLong).Length
        }
        if ($i -lt ($FreshnessFrames - 1) -and $FreshnessIntervalSeconds -gt 0) {
            Start-Sleep -Seconds $FreshnessIntervalSeconds
        }
    }
    return $hashes
}

function Invoke-FreshnessAnalysis {
    param([string]$Label)
    $dir = Join-Path $OutDir $Label
    $shotDir = Join-Path $dir "screenshots"
    $analysisPath = Join-Path $dir "freshness-analysis.json"
    if (-not (Test-Path -LiteralPath $freshnessAnalyzer)) {
        return [ordered]@{
            status = "skipped"
            reason = "analyzer-not-found"
            path = $analysisPath
        }
    }
    try {
        $analysisOutput = & python $freshnessAnalyzer `
            --sequence-dir $shotDir `
            --pattern ("{0}-frame-*.png" -f $Label) `
            --summary-out $analysisPath 2>&1
        $toolExitCode = $LASTEXITCODE
        if (Test-Path -LiteralPath $analysisPath) {
            return Get-Content -Raw -Path $analysisPath | ConvertFrom-Json
        }
        return [ordered]@{
            status = "tool-failed"
            reason = "missing-analysis-output"
            path = $analysisPath
            exitCode = $toolExitCode
            output = @($analysisOutput)
        }
    }
    catch {
        return [ordered]@{
            status = "tool-failed"
            reason = $_.Exception.Message
            path = $analysisPath
        }
    }
}

function Invoke-MetaPerfStaleAnalysis {
    param([object]$State)
    $dir = Join-Path $OutDir $State.label
    $logcatPath = Join-Path $dir "logcat.txt"
    $analysisPath = Join-Path $dir "meta-perf-stale-analysis.json"
    if (-not (Test-Path -LiteralPath $metaPerfStaleAnalyzer)) {
        return [ordered]@{
            status = "skipped"
            reason = "analyzer-not-found"
            path = $analysisPath
        }
    }
    if (-not (Test-Path -LiteralPath $logcatPath)) {
        return [ordered]@{
            status = "skipped"
            reason = "logcat-not-found"
            path = $analysisPath
        }
    }

    $arguments = @(
        $metaPerfStaleAnalyzer,
        "--logcat", $logcatPath,
        "--summary-out", $analysisPath
    )
    if ($State.processId) {
        $arguments += @("--app-pid", ([string]$State.processId))
    }

    try {
        $analysisOutput = & python @arguments 2>&1
        $toolExitCode = $LASTEXITCODE
        if (Test-Path -LiteralPath $analysisPath) {
            return Get-Content -Raw -Path $analysisPath | ConvertFrom-Json
        }
        return [ordered]@{
            status = "tool-failed"
            reason = "missing-analysis-output"
            path = $analysisPath
            exitCode = $toolExitCode
            output = @($analysisOutput)
        }
    }
    catch {
        return [ordered]@{
            status = "tool-failed"
            reason = $_.Exception.Message
            path = $analysisPath
        }
    }
}

function Invoke-CameraTextureLaneContractAnalysis {
    $analysisDir = Join-Path $OutDir "camera-texture-lane-analysis"
    $contractsPath = Join-Path $analysisDir "camera-texture-lane-contracts.jsonl"
    $summaryPath = Join-Path $analysisDir "camera-texture-lane-contract-summary.json"
    $stdoutPath = Join-Path $analysisDir "camera-texture-lane-builder-stdout.txt"
    $errorPath = Join-Path $analysisDir "camera-texture-lane-builder-error.txt"
    New-Item -ItemType Directory -Force -Path $analysisDir | Out-Null
    foreach ($path in @($stdoutPath, $errorPath)) {
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Force
        }
    }
    if (-not (Test-Path -LiteralPath $cameraTextureLaneContractBuilder)) {
        return [ordered]@{
            schema = "rusty.quest.makepad-camera-device-gate.camera-texture-lane-analysis.v1"
            status = "skipped"
            reason = "builder-not-found"
            outDir = $analysisDir
            contractsJsonl = $contractsPath
            summaryJson = $summaryPath
        }
    }

    try {
        $analysisOutput = @(& python $cameraTextureLaneContractBuilder $OutDir --out-dir $analysisDir 2>&1 |
            ForEach-Object { [string]$_ })
        $toolExitCode = $LASTEXITCODE
        $analysisOutput | Set-Content -Path $stdoutPath -Encoding UTF8
        $summary = $null
        if (Test-Path -LiteralPath $summaryPath) {
            $summary = Get-Content -Raw -LiteralPath $summaryPath | ConvertFrom-Json
        }
        $status = if ($toolExitCode -eq 0 -and $null -ne $summary) { "ok" } else { "tool-failed" }
        return [ordered]@{
            schema = "rusty.quest.makepad-camera-device-gate.camera-texture-lane-analysis.v1"
            status = $status
            outDir = $analysisDir
            contractsJsonl = $contractsPath
            summaryJson = $summaryPath
            stdout = $stdoutPath
            exitCode = $toolExitCode
            summary = $summary
        }
    }
    catch {
        @("camera texture lane contract analysis failed", $_.Exception.Message) |
            Set-Content -Path $errorPath -Encoding UTF8
        return [ordered]@{
            schema = "rusty.quest.makepad-camera-device-gate.camera-texture-lane-analysis.v1"
            status = "tool-failed"
            reason = $_.Exception.Message
            outDir = $analysisDir
            contractsJsonl = $contractsPath
            summaryJson = $summaryPath
            stdout = $stdoutPath
            error = $errorPath
        }
    }
}

function Wait-BrokerH264TextureReady {
    param([datetime]$LaunchStartedAt)
    if (-not ($UseBrokerH264Synthetic -or $UseBrokerH264Camera)) {
        return $null
    }

    $deadline = (Get-Date).AddSeconds($BrokerH264ReadyTimeoutSeconds)
    $state = $null
    do {
        Start-Sleep -Milliseconds $ReadyPollIntervalMs
        $state = Capture-LaunchState -Label "broker-h264-ready-poll" -LaunchStartedAt $LaunchStartedAt
        if ($state.brokerH264DecodedTextureReady) {
            return $state
        }
    } while ((Get-Date) -lt $deadline)
    return $state
}

if (-not $OutDir) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutDir = Join-Path (Get-Location) "artifacts/makepad-camera-device-gate-$stamp"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$script:gateTimingPath = Join-Path $OutDir "device-gate-timings.jsonl"
$script:gateTimingSummaryPath = Join-Path $OutDir "device-gate-timing-summary.json"
$script:gateStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$script:gateTimingRecords = [System.Collections.Generic.List[object]]::new()
$effectiveProjectionRuntimeReadback = if ($ProjectionRuntimeReadback -eq "warn" -and $UseResolvedProjectionRuntime) { "required" } else { $ProjectionRuntimeReadback }
$directCameraColorStatus = if ($DirectCameraTexturePath -eq "hardware-buffer-external") {
    "experimental-hardware-buffer-external-combined-immutable-default-sampler-ycbcr-candidate"
} else {
    "accepted-cpu-yuv-reference"
}

Invoke-GateTimedStep -Step "adb-devices" -Action {
    Invoke-Adb -Arguments @("devices") | Set-Content -Path (Join-Path $OutDir "adb-devices.txt") -Encoding UTF8
}
$projectionPropertyHygieneSummary = Invoke-GateTimedStep -Step "projection-property-hygiene" -Action {
    Invoke-RustyQuestMakepadProjectionPropertyHygiene `
        -Adb "adb" `
        -Serial $Serial `
        -Mode $ProjectionPropertyHygiene `
        -OutputPath (Join-Path $OutDir "projection-property-hygiene.json")
}
$preLaunchForceStopSummary = Invoke-GateTimedStep -Step "prelaunch-force-stop-packages" -Action {
    Stop-PreLaunchPackages
}
Invoke-GateTimedStep -Step "install-apk" -Action { Install-Apk }
Invoke-GateTimedStep -Step "grant-runtime-permissions" -Action { Grant-RuntimePermissions }
$oculusPerformanceProfile = Invoke-GateTimedStep -Step "set-oculus-performance-profile" -Action { Set-MakepadOculusPerformanceProfile }
Invoke-GateTimedStep -Step "set-projection-target-profile" -Action { Set-MakepadProjectionTargetProfile }
Invoke-GateTimedStep -Step "set-broker-h264-profile" -Action { Set-MakepadBrokerH264Profile }
Invoke-GateTimedStep -Step "prelaunch-state-capture" -Action {
    Save-Adb -Arguments @("shell", "dumpsys", "power") -Path (Join-Path $OutDir "power-before-launch.txt")
    Save-Adb -Arguments @("shell", "getprop") -Path (Join-Path $OutDir "getprop-before-launch.txt")
}

$attempts = @()
$freshnessAnalysis = $null
$metaPerfStaleAnalysis = $null
if ($PreferDirectVrActivity) {
    $attempts += Invoke-GateTimedStep -Step "start-direct-vr-attempt-1" -Action {
        Start-ActivityAndProbe -Label "direct-vr-attempt-1" -Activity $XrActivity -ForceStopFirst -VrIntent
    }
    if (-not $attempts[-1].ready) {
        $attempts += Invoke-GateTimedStep -Step "start-launcher-fallback-1" -Action {
            Start-ActivityAndProbe -Label "launcher-fallback-1" -Activity $LauncherActivity -LauncherIntent
        }
    }
}
else {
    $attempts += Invoke-GateTimedStep -Step "start-launcher-attempt-1" -Action {
        Start-ActivityAndProbe -Label "launcher-attempt-1" -Activity $LauncherActivity -ForceStopFirst -LauncherIntent
    }
    if (-not $attempts[-1].ready) {
        $attempts += Invoke-GateTimedStep -Step "start-launcher-attempt-2" -Action {
            Start-ActivityAndProbe -Label "launcher-attempt-2" -Activity $LauncherActivity -LauncherIntent
        }
    }
    if (-not $attempts[-1].ready -and -not $SkipDirectXrFallback) {
        $attempts += Invoke-GateTimedStep -Step "start-direct-vr-fallback" -Action {
            Start-ActivityAndProbe -Label "direct-vr-fallback" -Activity $XrActivity -VrIntent
        }
    }
}

$finalLabel = $attempts[-1].label
if ($attempts[-1].ready) {
    $launchStartedAt = if ($attempts[-1].launchedAt) {
        [datetime]::Parse(
            $attempts[-1].launchedAt,
            [System.Globalization.CultureInfo]::InvariantCulture,
            [System.Globalization.DateTimeStyles]::RoundtripKind)
    } else {
        Get-Date
    }
    $brokerReadyState = Invoke-GateTimedStep -Step "broker-h264-texture-readiness" -Action {
        Wait-BrokerH264TextureReady -LaunchStartedAt $launchStartedAt
    }
    if ($brokerReadyState) {
        $attempts += $brokerReadyState
    }
    if ($UseFixedSampleWindow) {
        Invoke-GateTimedStep -Step "fixed-sample-window" -Action {
            Start-Sleep -Seconds ([Math]::Max(0, $SampleSeconds - ($FreshnessFrames * $FreshnessIntervalSeconds)))
        }
    }
    elseif ($ReadySettleMs -gt 0) {
        Invoke-GateTimedStep -Step "ready-settle" -Action {
            Start-Sleep -Milliseconds $ReadySettleMs
        }
    }
    $frames = Invoke-GateTimedStep -Step "capture-freshness-frames" -Action {
        Capture-FreshnessFrames -Label "$finalLabel-final"
    }
    $freshnessAnalysis = Invoke-GateTimedStep -Step "analyze-freshness-frames" -Action {
        Invoke-FreshnessAnalysis -Label "$finalLabel-final"
    }
    $finalState = Invoke-GateTimedStep -Step "capture-final-state" -Action {
        Capture-LaunchState -Label "$finalLabel-final" -LaunchStartedAt $launchStartedAt
    }
    $attempts += $finalState
    $metaPerfStaleAnalysis = Invoke-GateTimedStep -Step "analyze-meta-perf-stale" -Action {
        Invoke-MetaPerfStaleAnalysis -State $finalState
    }
} else {
    $frames = @()
}

$readyAttempt = $attempts |
    Where-Object { $_.label -notlike "*-final" -and $_.ready } |
    Select-Object -First 1
$finalAttempt = $attempts[-1]
$projectionRuntimeManifestTotal = 0
$resolvedProjectionRuntimeEnabledMarkerTotal = 0
$projectionRuntimeNumericTypeIssueTotal = 0
$projectionRuntimeNumericTypeIssues = @()
foreach ($attempt in $attempts) {
    if ($null -ne $attempt.projectionRuntimeManifestCount) {
        $projectionRuntimeManifestTotal += [int]$attempt.projectionRuntimeManifestCount
    }
    if ($null -ne $attempt.resolvedProjectionRuntimeEnabledMarkerCount) {
        $resolvedProjectionRuntimeEnabledMarkerTotal += [int]$attempt.resolvedProjectionRuntimeEnabledMarkerCount
    }
    if ($null -ne $attempt.projectionRuntimeNumericTypeIssueCount) {
        $projectionRuntimeNumericTypeIssueTotal += [int]$attempt.projectionRuntimeNumericTypeIssueCount
    }
    if ($null -ne $attempt.projectionRuntimeNumericTypeIssues) {
        $projectionRuntimeNumericTypeIssues += @($attempt.projectionRuntimeNumericTypeIssues)
    }
}
$projectionRuntimeGateFailures = @()
if ($UseResolvedProjectionRuntime) {
    if ($projectionRuntimeManifestTotal -le 0) {
        $projectionRuntimeGateFailures += "missing projection runtime manifest"
    }
    if ($resolvedProjectionRuntimeEnabledMarkerTotal -le 0) {
        $projectionRuntimeGateFailures += "missing resolved projection runtime consumption marker"
    }
    if ($projectionRuntimeNumericTypeIssueTotal -gt 0) {
        $projectionRuntimeGateFailures += "numeric projection fields resolved as bool"
    }
}
$projectionRuntimeReadbackSummary = Invoke-GateTimedStep -Step "projection-runtime-readback" -Action {
    Invoke-ProjectionRuntimeReadbackValidation -Attempts $attempts -Mode $effectiveProjectionRuntimeReadback
}
if ($effectiveProjectionRuntimeReadback -eq "required" -and $projectionRuntimeReadbackSummary.status -ne "ok") {
    $projectionRuntimeGateFailures += "projection runtime readback validation failed"
}
$cameraTextureLaneAnalysis = Invoke-GateTimedStep -Step "camera-texture-lane-contract-analysis" -Action {
    Invoke-CameraTextureLaneContractAnalysis
}
$freshnessFrameCount = @($frames).Count
$uniqueFreshnessHashes = @($frames.sha256 | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique).Count
$freshnessGateFailures = @()
if ($readyAttempt -and $freshnessFrameCount -lt $FreshnessFrames) {
    $freshnessGateFailures += "captured $freshnessFrameCount of $FreshnessFrames requested freshness frames"
}
if ($readyAttempt -and $uniqueFreshnessHashes -lt $FreshnessRequiredUniqueHashes) {
    $freshnessGateFailures += "captured $uniqueFreshnessHashes unique freshness hashes; required $FreshnessRequiredUniqueHashes"
}
$freshnessStatus = if (-not $readyAttempt) {
    "skipped"
} elseif ($freshnessGateFailures.Count -gt 0) {
    "stale"
} elseif ($null -ne $freshnessAnalysis -and $freshnessAnalysis.status -eq "stale") {
    "stale"
} else {
    "ok"
}
$metaPerfStaleStatus = if (-not $readyAttempt) {
    "skipped"
} elseif ($null -eq $metaPerfStaleAnalysis) {
    "skipped"
} else {
    [string]$metaPerfStaleAnalysis.status
}
$metaPerfStaleGateFailures = @()
if ($readyAttempt -and $metaPerfStaleStatus -eq "stale") {
    $metaPerfStaleReasons = @($metaPerfStaleAnalysis.reasons) |
        Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }
    $metaPerfStaleGateFailures += "Meta performance stale analysis failed: $($metaPerfStaleReasons -join ', ')"
}
elseif ($readyAttempt -and $metaPerfStaleStatus -eq "tool-failed") {
    $metaPerfStaleGateFailures += "Meta performance stale analysis tool failed"
}

function Get-AttemptPropertyValue {
    param(
        [object]$Attempt,
        [string]$PropertyName
    )
    if ($null -eq $Attempt) {
        return $null
    }
    if ($Attempt -is [System.Collections.IDictionary]) {
        if ($Attempt.Contains($PropertyName)) {
            return $Attempt[$PropertyName]
        }
        return $null
    }
    $property = $Attempt.PSObject.Properties[$PropertyName]
    if ($null -ne $property) {
        return $property.Value
    }
    return $null
}

function Get-AttemptIntPropertyTotal {
    param(
        [object[]]$Attempts,
        [string]$PropertyName
    )
    $total = 0
    foreach ($attempt in @($Attempts)) {
        $value = Get-AttemptPropertyValue -Attempt $attempt -PropertyName $PropertyName
        if ($null -ne $value -and -not [string]::IsNullOrWhiteSpace([string]$value)) {
            $total += [int]$value
        }
    }
    return $total
}

function Get-AttemptIntPropertyMax {
    param(
        [object[]]$Attempts,
        [string]$PropertyName
    )
    $maxValue = 0
    foreach ($attempt in @($Attempts)) {
        $value = Get-AttemptPropertyValue -Attempt $attempt -PropertyName $PropertyName
        if ($null -ne $value -and -not [string]::IsNullOrWhiteSpace([string]$value)) {
            $maxValue = [Math]::Max($maxValue, [int]$value)
        }
    }
    return $maxValue
}

function Test-AttemptBoolPropertyAny {
    param(
        [object[]]$Attempts,
        [string]$PropertyName
    )
    foreach ($attempt in @($Attempts)) {
        $value = Get-AttemptPropertyValue -Attempt $attempt -PropertyName $PropertyName
        if ($null -eq $value) {
            continue
        }
        if ($value -is [bool]) {
            if ($value) {
                return $true
            }
            continue
        }
        if ([string]$value -eq "true") {
            return $true
        }
    }
    return $false
}

$brokerH264StereoProjectionGateFailures = @()
$brokerH264ModeEnabled = [bool]($UseBrokerH264Camera -or $UseBrokerH264Synthetic)
$brokerH264StereoProjectionAttempt = if ($finalAttempt) { $finalAttempt } else { $readyAttempt }
$frameAdoptionAttempt = if ($finalAttempt) { $finalAttempt } else { $readyAttempt }
$brokerH264StereoProjectionAttempts = @($attempts | Where-Object { $null -ne $_ })
$brokerH264DecodedTextureReadyAny = Test-AttemptBoolPropertyAny `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264DecodedTextureReady"
$brokerH264PreparedTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264PreparedMarkerCount"
$brokerH264TextureUpdateTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264TextureUpdateMarkerCount"
$brokerH264YuvTextureUpdateTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264YuvTextureUpdateMarkerCount"
$brokerH264HardwareBufferTextureUpdateTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264HardwareBufferTextureUpdateMarkerCount"
$brokerH264HardwareBufferFrameTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264HardwareBufferFrameMarkerCount"
$brokerH264DecodeErrorTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264DecodeErrorMarkerCount"
$brokerH264LeftTextureUpdateMax = Get-AttemptIntPropertyMax `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264LeftTextureUpdateMax"
$brokerH264RightTextureUpdateMax = Get-AttemptIntPropertyMax `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264RightTextureUpdateMax"
$brokerH264PairedMarkerTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "pairedCameraFrameCadenceMarkerCount"
$brokerH264ProjectionMappingReadyTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "projectionMappingReadyCadenceMarkerCount"
$brokerH264FrameAdoptionTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "frameAdoptionMarkerCount"
$brokerH264FrameAdoptionPoseMatchedTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "frameAdoptionPoseMatchedMarkerCount"
$brokerH264LeftRequestedCameraHeaderTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264LeftRequestedCameraHeaderMarkerCount"
$brokerH264RightRequestedCameraHeaderTotal = Get-AttemptIntPropertyTotal `
    -Attempts $brokerH264StereoProjectionAttempts `
    -PropertyName "brokerH264RightRequestedCameraHeaderMarkerCount"
$brokerH264HardwareBufferOutputProof = [bool](
    $BrokerH264DecodeOutputMode -eq "hardware-buffer" -and
    $brokerH264PreparedTotal -gt 0 -and
    $brokerH264HardwareBufferFrameTotal -ge ($BrokerH264MinimumPerEyeTextureUpdates * 2) -and
    $brokerH264DecodeErrorTotal -eq 0)
$brokerH264DecodedTextureReadyAggregate = [bool](
    $brokerH264DecodedTextureReadyAny -or
    ($brokerH264PreparedTotal -gt 0 -and $brokerH264TextureUpdateTotal -gt 0 -and $brokerH264DecodeErrorTotal -eq 0) -or
    $brokerH264HardwareBufferOutputProof)
if ($brokerH264StereoProjectionAttempt -and $RequireBrokerH264StereoProjection -and $brokerH264ModeEnabled) {
    if ($UseBrokerH264Camera) {
        if ($brokerH264LeftRequestedCameraHeaderTotal -lt 1) {
            $brokerH264StereoProjectionGateFailures += "left broker stream header did not confirm requested cameraId=$BrokerH264LeftCameraId"
        }
        if ($brokerH264RightRequestedCameraHeaderTotal -lt 1) {
            $brokerH264StereoProjectionGateFailures += "right broker stream header did not confirm requested cameraId=$BrokerH264RightCameraId"
        }
    }
    if (-not $brokerH264DecodedTextureReadyAggregate) {
        $brokerH264StereoProjectionGateFailures += "broker H.264 decoded texture was not marked ready"
    }
    if ($brokerH264LeftTextureUpdateMax -lt $BrokerH264MinimumPerEyeTextureUpdates) {
        $brokerH264StereoProjectionGateFailures += "left texture update max $brokerH264LeftTextureUpdateMax below required $BrokerH264MinimumPerEyeTextureUpdates"
    }
    if ($brokerH264RightTextureUpdateMax -lt $BrokerH264MinimumPerEyeTextureUpdates) {
        $brokerH264StereoProjectionGateFailures += "right texture update max $brokerH264RightTextureUpdateMax below required $BrokerH264MinimumPerEyeTextureUpdates"
    }
    if ($brokerH264PairedMarkerTotal -lt 1) {
        $brokerH264StereoProjectionGateFailures += "no pairedLeftRightCameraFrames=true cadence marker"
    }
    if ($brokerH264ProjectionMappingReadyTotal -lt 1) {
        $brokerH264StereoProjectionGateFailures += "no projectionMappingReady=true cadence marker"
    }
    if ($brokerH264FrameAdoptionTotal -lt 1) {
        $brokerH264StereoProjectionGateFailures += "no adopted stereo camera frame marker"
    }
    if ($brokerH264FrameAdoptionPoseMatchedTotal -lt 1) {
        $brokerH264StereoProjectionGateFailures += "no adopted stereo camera frame marker with XR pose match"
    }
}
$resolvedBrokerH264ProjectionGeometryProfile = if ($BrokerH264ProjectionGeometryProfile -and $BrokerH264ProjectionGeometryProfile.Trim().Length -gt 0) {
    $BrokerH264ProjectionGeometryProfile.Trim()
}
elseif ($UseBrokerH264Camera) {
    $CameraProjectionGeometryProfile
}
elseif ($UseBrokerH264Synthetic) {
    $BrokerH264SyntheticProjectionProfile
}
else {
    $CameraProjectionGeometryProfile
}
$resolvedBrokerH264SyntheticProjectionProfile = if ($UseBrokerH264Camera -or -not ($UseBrokerH264Synthetic -or $UseBrokerH264Camera)) {
    $resolvedBrokerH264ProjectionGeometryProfile
}
else {
    $BrokerH264SyntheticProjectionProfile
}
$gateTimingSummary = New-GateTimingSummary
$gateTimingSummary | ConvertTo-Json -Depth 8 | Set-Content -Path $script:gateTimingSummaryPath -Encoding UTF8

$summary = [ordered]@{
    schema = "rusty.quest.makepad-camera-device-gate.v1"
    capturedAt = (Get-Date).ToString("o")
    serial = $Serial
    packageName = $PackageName
    apk = $Apk
    preferDirectVrActivity = [bool]$PreferDirectVrActivity
    useBrokerH264Synthetic = [bool]$UseBrokerH264Synthetic
    useBrokerH264Camera = [bool]$UseBrokerH264Camera
    brokerH264SourceMode = if ($UseBrokerH264Camera) { "broker-camera" } elseif ($UseBrokerH264Synthetic) { "broker-synthetic" } else { "disabled" }
    cameraProjectionMode = $CameraProjectionMode
    cameraProjectionGeometryProfile = $CameraProjectionGeometryProfile
    cameraSourceSamplingMode = $CameraSourceSamplingMode
    cameraTargetScreenUvRect = $CameraTargetScreenUvRect
    cameraLeftTargetScreenUvRect = $CameraLeftTargetScreenUvRect
    cameraRightTargetScreenUvRect = $CameraRightTargetScreenUvRect
    brokerH264ProjectionGeometryProfile = $resolvedBrokerH264ProjectionGeometryProfile
    brokerH264SyntheticProjectionProfile = $resolvedBrokerH264SyntheticProjectionProfile
    brokerH264DecodeOutputMode = $BrokerH264DecodeOutputMode
    projectionBorderPolicy = $ProjectionBorderPolicy
    xrRenderScale = [double]$XrRenderScale
    nativePassthroughRequested = [bool]($EnableNativePassthrough -or $ProjectionBorderPolicy -eq "passthrough-underlay" -or $ProjectionAreaOpacity -lt 1.0 -or $ProjectionBorderOpacity -lt 1.0 -or $ProjectionAlphaMode -ne "fixed")
    xrDisplayRefreshHz = $XrDisplayRefreshHz
    oculusCpuLevel = $OculusCpuLevel
    oculusGpuLevel = $OculusGpuLevel
    oculusFoveationLevel = $OculusFoveationLevel
    oculusFoveationDynamic = $OculusFoveationDynamic
    oculusPerformanceProfile = $oculusPerformanceProfile
    projectionAreaOpacity = $ProjectionAreaOpacity
    projectionBorderOpacity = $ProjectionBorderOpacity
    projectionTargetOffsetXUv = [double]$ProjectionTargetOffsetXUv
    projectionTargetOffsetYUv = [double]$ProjectionTargetOffsetYUv
    projectionTargetScale = [double]$ProjectionTargetScale
    projectionTargetJoystickControls = $ProjectionTargetJoystickControls
    projectionAlphaMode = $ProjectionAlphaMode
    projectionAlphaScale = $ProjectionAlphaScale
    projectionAlphaBias = $ProjectionAlphaBias
    useResolvedProjectionRuntime = [bool]$UseResolvedProjectionRuntime
    mediaProjection = [bool]$MediaProjection
    startupTimeoutSeconds = $StartupTimeoutSeconds
    readyPollIntervalMs = $ReadyPollIntervalMs
    readySettleMs = $ReadySettleMs
    useFixedSampleWindow = [bool]$UseFixedSampleWindow
    sampleSeconds = $SampleSeconds
    directCameraTexturePath = $DirectCameraTexturePath
    directCameraColorStatus = $directCameraColorStatus
    directCameraHardwareBufferExternalRequested = [bool]($DirectCameraTexturePath -eq "hardware-buffer-external")
    skipPreLaunchForceStopPackages = [bool]$SkipPreLaunchForceStopPackages
    preLaunchForceStopPackages = $PreLaunchForceStopPackages
    preLaunchForceStop = $preLaunchForceStopSummary
    projectionPropertyHygiene = $projectionPropertyHygieneSummary
    projectionRuntimeReadbackMode = $effectiveProjectionRuntimeReadback
    projectionRuntimeReadback = $projectionRuntimeReadbackSummary
    timingJsonl = $script:gateTimingPath
    timingSummary = $script:gateTimingSummaryPath
    timing = [ordered]@{
        totalElapsedMs = $gateTimingSummary.totalElapsedMs
        jsonl = $script:gateTimingPath
        summary = $script:gateTimingSummaryPath
    }
    mediaProjectionPort = $MediaProjectionPort
    mediaProjectionWidth = $MediaProjectionWidth
    mediaProjectionHeight = $MediaProjectionHeight
    mediaProjectionDelayMs = $MediaProjectionDelayMs
    processingLayer = $ProcessingLayer
    projectionSampleMode = $ProjectionSampleMode
    blurRadiusPx = $BlurRadiusPx
    peripheralStretchMode = $PeripheralStretchMode
    peripheralStretchCoreScale = $PeripheralStretchCoreScale
    peripheralStretchEdgeInsetUv = $PeripheralStretchEdgeInsetUv
    peripheralStretchMaxInsetUv = $PeripheralStretchMaxInsetUv
    peripheralStretchCurve = $PeripheralStretchCurve
    peripheralStretchInnerBlendUv = $PeripheralStretchInnerBlendUv
    peripheralStretchBlendCurve = $PeripheralStretchBlendCurve
    peripheralStretchBlendMode = $PeripheralStretchBlendMode
    peripheralStretchCornerMode = $PeripheralStretchCornerMode
    peripheralStretchDebug = $PeripheralStretchDebug
    projectionAreaDiagnostic = [double]$ProjectionAreaDiagnostic
    runConfiguration = [ordered]@{
        xrRenderScale = [double]$XrRenderScale
        projectionBorderPolicy = $ProjectionBorderPolicy
        processingLayer = $ProcessingLayer
        projectionSampleMode = $ProjectionSampleMode
        blurRadiusPx = [double]$BlurRadiusPx
        peripheralStretchMode = $PeripheralStretchMode
        peripheralStretchCoreScale = [double]$PeripheralStretchCoreScale
        peripheralStretchEdgeInsetUv = [double]$PeripheralStretchEdgeInsetUv
        peripheralStretchMaxInsetUv = [double]$PeripheralStretchMaxInsetUv
        peripheralStretchCurve = [double]$PeripheralStretchCurve
        peripheralStretchInnerBlendUv = [double]$PeripheralStretchInnerBlendUv
        peripheralStretchBlendCurve = [double]$PeripheralStretchBlendCurve
        peripheralStretchBlendMode = $PeripheralStretchBlendMode
        peripheralStretchCornerMode = $PeripheralStretchCornerMode
        peripheralStretchDebug = $PeripheralStretchDebug
        projectionTargetOffsetXUv = [double]$ProjectionTargetOffsetXUv
        projectionTargetOffsetYUv = [double]$ProjectionTargetOffsetYUv
        projectionTargetScale = [double]$ProjectionTargetScale
        projectionTargetJoystickControls = $ProjectionTargetJoystickControls
        projectionAreaDiagnostic = [double]$ProjectionAreaDiagnostic
        directCameraTexturePath = $DirectCameraTexturePath
        cameraProjectionMode = $CameraProjectionMode
        cameraProjectionGeometryProfile = $CameraProjectionGeometryProfile
        cameraSourceSamplingMode = $CameraSourceSamplingMode
        cameraTargetScreenUvRect = $CameraTargetScreenUvRect
        cameraLeftTargetScreenUvRect = $CameraLeftTargetScreenUvRect
        cameraRightTargetScreenUvRect = $CameraRightTargetScreenUvRect
        brokerH264DecodeOutputMode = $BrokerH264DecodeOutputMode
        useResolvedProjectionRuntime = [bool]$UseResolvedProjectionRuntime
        sampleSeconds = $SampleSeconds
        useFixedSampleWindow = [bool]$UseFixedSampleWindow
    }
    brokerH264FrameRateHz = $BrokerH264FrameRateHz
    brokerH264LeftCameraId = $BrokerH264LeftCameraId
    brokerH264RightCameraId = $BrokerH264RightCameraId
    launchReady = [bool]$readyAttempt
    recoveredBy = if ($readyAttempt) { $readyAttempt.label } else { "none" }
    attempts = $attempts
    projectionRuntimeManifestTotal = $projectionRuntimeManifestTotal
    resolvedProjectionRuntimeEnabledMarkerTotal = $resolvedProjectionRuntimeEnabledMarkerTotal
    projectionRuntimeNumericTypeIssueTotal = $projectionRuntimeNumericTypeIssueTotal
    projectionRuntimeNumericTypeIssues = $projectionRuntimeNumericTypeIssues
    projectionRuntimeGateFailureCount = $projectionRuntimeGateFailures.Count
    projectionRuntimeGateFailures = $projectionRuntimeGateFailures
    freshnessFrameCount = $freshnessFrameCount
    freshnessRequiredUniqueHashes = $FreshnessRequiredUniqueHashes
    uniqueFreshnessHashes = $uniqueFreshnessHashes
    freshnessStatus = $freshnessStatus
    freshnessGateFailureCount = $freshnessGateFailures.Count
    freshnessGateFailures = $freshnessGateFailures
    freshnessAnalysis = $freshnessAnalysis
    metaPerfStaleStatus = $metaPerfStaleStatus
    metaPerfStaleGateFailureCount = $metaPerfStaleGateFailures.Count
    metaPerfStaleGateFailures = $metaPerfStaleGateFailures
    brokerH264StereoProjectionRequired = [bool]$RequireBrokerH264StereoProjection
    brokerH264MinimumPerEyeTextureUpdates = $BrokerH264MinimumPerEyeTextureUpdates
    brokerH264StereoProjectionStatus = if (-not $RequireBrokerH264StereoProjection -or -not $brokerH264ModeEnabled -or -not $brokerH264StereoProjectionAttempt) { "skipped" } elseif ($brokerH264StereoProjectionGateFailures.Count -eq 0) { "ok" } else { "failed" }
    brokerH264StereoProjectionAttemptLabel = if ($brokerH264StereoProjectionAttempt) { $brokerH264StereoProjectionAttempt.label } else { "none" }
    brokerH264StereoProjectionGateFailureCount = $brokerH264StereoProjectionGateFailures.Count
    brokerH264StereoProjectionGateFailures = $brokerH264StereoProjectionGateFailures
    frameAdoptionAttemptLabel = if ($frameAdoptionAttempt) { $frameAdoptionAttempt.label } else { "none" }
    frameAdoptionMarkerCount = if ($frameAdoptionAttempt) { [int]$frameAdoptionAttempt.frameAdoptionMarkerCount } else { 0 }
    frameAdoptionPoseMatchedMarkerCount = if ($frameAdoptionAttempt) { [int]$frameAdoptionAttempt.frameAdoptionPoseMatchedMarkerCount } else { 0 }
    frameAdoptionCloseTimestampMatchMarkerCount = if ($frameAdoptionAttempt) { [int]$frameAdoptionAttempt.frameAdoptionCloseTimestampMatchMarkerCount } else { 0 }
    frameAdoptionTimingGapMarkerCount = if ($frameAdoptionAttempt) { [int]$frameAdoptionAttempt.frameAdoptionTimingGapMarkerCount } else { 0 }
    latestFrameAdoptionMarker = if ($frameAdoptionAttempt) { [string]$frameAdoptionAttempt.latestFrameAdoptionMarker } else { "" }
    metaPerfStaleAnalysis = $metaPerfStaleAnalysis
    makepadFrameFlow = $metaPerfStaleAnalysis.makepadFrameFlow
    cameraTextureLaneAnalysis = $cameraTextureLaneAnalysis
    cameraTextureLaneSummary = $cameraTextureLaneAnalysis.summary
    brokerH264DecodedTextureReadyAggregate = [bool]$brokerH264DecodedTextureReadyAggregate
    brokerH264DecodedTextureReadyAny = [bool]$brokerH264DecodedTextureReadyAny
    brokerH264PreparedMarkerTotal = $brokerH264PreparedTotal
    brokerH264TextureUpdateMarkerTotal = $brokerH264TextureUpdateTotal
    brokerH264YuvTextureUpdateMarkerTotal = $brokerH264YuvTextureUpdateTotal
    brokerH264HardwareBufferTextureUpdateMarkerTotal = $brokerH264HardwareBufferTextureUpdateTotal
    brokerH264HardwareBufferFrameMarkerTotal = $brokerH264HardwareBufferFrameTotal
    brokerH264DecodeErrorMarkerTotal = $brokerH264DecodeErrorTotal
    brokerH264LeftTextureUpdateMaxAggregate = $brokerH264LeftTextureUpdateMax
    brokerH264RightTextureUpdateMaxAggregate = $brokerH264RightTextureUpdateMax
    brokerH264PairedCameraFrameCadenceMarkerTotal = $brokerH264PairedMarkerTotal
    brokerH264ProjectionMappingReadyCadenceMarkerTotal = $brokerH264ProjectionMappingReadyTotal
    brokerH264FrameAdoptionMarkerTotal = $brokerH264FrameAdoptionTotal
    brokerH264FrameAdoptionPoseMatchedMarkerTotal = $brokerH264FrameAdoptionPoseMatchedTotal
    brokerH264LeftRequestedCameraHeaderMarkerTotal = $brokerH264LeftRequestedCameraHeaderTotal
    brokerH264RightRequestedCameraHeaderMarkerTotal = $brokerH264RightRequestedCameraHeaderTotal
    freshnessFrames = $frames
}
$summary | ConvertTo-Json -Depth 7 | Set-Content -Path (Join-Path $OutDir "summary.json") -Encoding UTF8
$summary | ConvertTo-Json -Depth 7
if ($projectionRuntimeGateFailures.Count -gt 0) {
    throw "resolved projection runtime device gate failed: $($projectionRuntimeGateFailures -join '; ')"
}
if ($freshnessGateFailures.Count -gt 0) {
    throw "freshness gate failed: $($freshnessGateFailures -join '; ')"
}
if ($metaPerfStaleGateFailures.Count -gt 0) {
    throw "meta performance stale gate failed: $($metaPerfStaleGateFailures -join '; ')"
}
if ($brokerH264StereoProjectionGateFailures.Count -gt 0) {
    throw "broker H.264 stereo projection gate failed: $($brokerH264StereoProjectionGateFailures -join '; ')"
}





