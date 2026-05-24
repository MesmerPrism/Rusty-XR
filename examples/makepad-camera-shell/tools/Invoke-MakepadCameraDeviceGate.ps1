param(
    [Parameter(Mandatory = $true)]
    [string]$Serial,

    [Parameter(Mandatory = $true)]
    [string]$Apk,

    [string]$PackageName = "io.github.mesmerprism.rustyxr.makepad.camera",
    [string]$LauncherActivity = ("." + "Makepad" + "App"),
    [string]$XrActivity = ("." + "Makepad" + "App" + "Xr"),
    [string]$OutDir = "",
    [int]$StartupTimeoutSeconds = 30,
    [int]$SampleSeconds = 90,
    [int]$FreshnessFrames = 6,
    [int]$FreshnessIntervalSeconds = 1,
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
    [string]$CameraProjectionGeometryProfile = "full-frame-diagnostic",
    [ValidateSet("head-anchored-virtual-camera", "camera-matched", "full-frame-diagnostic")]
    [string]$BrokerH264SyntheticProjectionProfile = "head-anchored-virtual-camera",
    [string]$BrokerH264LeftCameraId = "",
    [string]$BrokerH264RightCameraId = "",
    [int]$BrokerH264Width = 1280,
    [int]$BrokerH264Height = 1280,
    [int]$BrokerH264CaptureMs = 45000,
    [int]$BrokerH264MaxPackets = 0,
    [int]$BrokerH264BitrateBps = 6000000,
    [int]$BrokerH264FrameRateHz = 50,
    [int]$BrokerH264StreamTimeoutMs = 60000,
    [int]$BrokerH264DecodeTimeoutMs = 20000,
    [ValidateSet("solid-red", "passthrough-underlay")]
    [string]$ProjectionBorderPolicy = "solid-red",
    [ValidateSet("raw", "blur")]
    [string]$ProcessingLayer = "raw",
    [double]$BlurRadiusPx = 2.0,
    [double]$ProjectionScale = 1.0,
    [double]$ProjectionDepthMeters = 1.0,
    [double]$CameraPreviewFovYDegrees = [double]::NaN,
    [double]$CameraPreviewOffsetYMeters = [double]::NaN,
    [double]$CameraRawOverlayOverscan = [double]::NaN,
    [double]$XrRenderScale = 1.0,
    [double]$ProjectionAreaOffsetXUv = 0.0,
    [double]$ProjectionAreaOffsetLeftUv = [double]::NaN,
    [double]$ProjectionAreaOffsetRightUv = [double]::NaN,
    [double]$ProjectionAreaOffsetYUv = 0.0,
    [double]$ProjectionAreaScaleX = 1.0,
    [double]$ProjectionAreaScaleY = 1.0,
    [double]$ProjectionAreaRadiusXUv = 0.5,
    [double]$ProjectionAreaRadiusYUv = 0.5,
    [double]$ProjectionAreaCornerRadiusUv = 0.0,
    [double]$ProjectionAreaOpacity = 1.0,
    [double]$ProjectionBorderOpacity = 1.0,
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
    [switch]$EnableNativePassthrough
)

$ErrorActionPreference = "Stop"
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\..\.."))
$projectionPropertyHygieneHelper = Join-Path $repoRoot "tools\quest-camera-profile\ProjectionPropertyHygiene.ps1"
$projectionRuntimeReadbackValidator = Join-Path $repoRoot "tools\quest-camera-profile\Validate-ProjectionRuntimeReadback.py"
. $projectionPropertyHygieneHelper

function Invoke-Adb {
    param([string[]]$Arguments)
    & adb -s $Serial @Arguments
}

function Save-Adb {
    param(
        [string[]]$Arguments,
        [string]$Path
    )
    Invoke-Adb -Arguments $Arguments 2>&1 | Set-Content -Path $Path -Encoding UTF8
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
    $tempPath = Join-Path ([System.IO.Path]::GetTempPath()) ("rustyxr-makepad-{0}.tmp" -f [guid]::NewGuid())
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
        if ($line -notmatch "RUSTY_XR_PROJECTION_RUNTIME_MANIFEST") {
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
            schemaVersion = "rusty.xr.projection-runtime-readback.v1"
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
            schemaVersion = "rusty.xr.projection-runtime-readback.v1"
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
            schemaVersion = "rusty.xr.projection-runtime-readback.v1"
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
        schemaVersion = "rusty.xr.projection-runtime-readback.v1"
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
        "debug.rustyxr.makepad.broker.h264.enabled" = if ($brokerRequested) { "true" } else { "false" }
        "debug.rustyxr.makepad.broker.h264.host" = $BrokerH264Host
        "debug.rustyxr.makepad.broker.h264.broker.port" = $BrokerH264BrokerPort
        "debug.rustyxr.makepad.broker.h264.stream.port" = $BrokerH264LeftStreamPort
        "debug.rustyxr.makepad.broker.h264.right.stream.port" = $BrokerH264RightStreamPort
        "debug.rustyxr.makepad.broker.h264.source.mode" = $sourceMode
        "debug.rustyxr.makepad.broker.h264.synthetic.pattern" = $BrokerH264SyntheticPattern
        "debug.rustyxr.makepad.broker.h264.projection.geometry.profile" = $projectionGeometryProfile
        "debug.rustyxr.makepad.broker.h264.synthetic.projection.profile" = $syntheticProjectionProfile
        "debug.rustyxr.makepad.broker.h264.left.camera.id" = $BrokerH264LeftCameraId
        "debug.rustyxr.makepad.broker.h264.right.camera.id" = $BrokerH264RightCameraId
        "debug.rustyxr.makepad.broker.h264.width" = $BrokerH264Width
        "debug.rustyxr.makepad.broker.h264.height" = $BrokerH264Height
        "debug.rustyxr.makepad.broker.h264.capture.ms" = $BrokerH264CaptureMs
        "debug.rustyxr.makepad.broker.h264.max.packets" = $BrokerH264MaxPackets
        "debug.rustyxr.makepad.broker.h264.bitrate.bps" = $BrokerH264BitrateBps
        "debug.rustyxr.makepad.broker.h264.frame.rate.hz" = $BrokerH264FrameRateHz
        "debug.rustyxr.makepad.broker.h264.stream.timeout.ms" = $BrokerH264StreamTimeoutMs
        "debug.rustyxr.makepad.broker.h264.decode.timeout.ms" = $BrokerH264DecodeTimeoutMs
        "debug.rustyxr.makepad.broker.h264.live.stream" = if ($brokerRequested) { "true" } else { "false" }
    }

    foreach ($entry in $props.GetEnumerator()) {
        Invoke-Adb -Arguments @("shell", "setprop", $entry.Key, [string]$entry.Value) | Out-Null
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
        "debug.rustyxr.makepad.native.passthrough.enabled" = $nativePassthrough
        "debug.rustyxr.makepad.projection.runtime.resolution.enabled" = if ($UseResolvedProjectionRuntime) { "true" } else { "false" }
        "debug.rustyxr.makepad.processing.layer" = $ProcessingLayer
        "debug.rustyxr.makepad.blur.radius.px" = (Format-InvariantDouble -Value $BlurRadiusPx)
        "debug.rustyxr.camera.projection.mode" = $CameraProjectionMode
        "debug.rustyxr.makepad.camera.projection.geometry.profile" = $CameraProjectionGeometryProfile
        "debug.rustyxr.projection.scale" = (Format-InvariantDouble -Value $ProjectionScale)
        "debug.rustyxr.projection.depth.meters" = (Format-InvariantDouble -Value $ProjectionDepthMeters)
        "debug.rustyxr.camera.preview.fov.y.degrees" = (Format-InvariantDouble -Value $previewFovYDegrees)
        "debug.rustyxr.camera.preview.offset.y.meters" = (Format-InvariantDouble -Value $previewOffsetYMeters)
        "debug.rustyxr.camera.raw.overlay.overscan" = (Format-InvariantDouble -Value $rawOverlayOverscan)
        "debug.rustyxr.xr.render.scale" = (Format-InvariantDouble -Value $XrRenderScale)
        "debug.rustyxr.projection.area.left.offset.x.uv" = (Format-InvariantDouble -Value $canonicalOffsetLeftUv)
        "debug.rustyxr.projection.area.right.offset.x.uv" = (Format-InvariantDouble -Value $canonicalOffsetRightUv)
        "debug.rustyxr.projection.area.offset.y.uv" = (Format-InvariantDouble -Value $offsetVerticalUv)
        "debug.rustyxr.projection.area.scale.x" = (Format-InvariantDouble -Value $ProjectionAreaScaleX)
        "debug.rustyxr.projection.area.scale.y" = (Format-InvariantDouble -Value $ProjectionAreaScaleY)
        "debug.rustyxr.projection.area.radius.x.uv" = (Format-InvariantDouble -Value $ProjectionAreaRadiusXUv)
        "debug.rustyxr.projection.area.radius.y.uv" = (Format-InvariantDouble -Value $ProjectionAreaRadiusYUv)
        "debug.rustyxr.projection.area.corner.radius.uv" = (Format-InvariantDouble -Value $ProjectionAreaCornerRadiusUv)
        "debug.rustyxr.projection.area.opacity" = (Format-InvariantDouble -Value $ProjectionAreaOpacity)
        "debug.rustyxr.projection.border.opacity" = (Format-InvariantDouble -Value $ProjectionBorderOpacity)
        "debug.rustyxr.projection.border.policy" = $ProjectionBorderPolicy
        "debug.rustyxr.projection.alpha.mode" = $ProjectionAlphaMode
        "debug.rustyxr.projection.alpha.scale" = (Format-InvariantDouble -Value $ProjectionAlphaScale)
        "debug.rustyxr.projection.alpha.bias" = (Format-InvariantDouble -Value $ProjectionAlphaBias)
    }

    foreach ($entry in $props.GetEnumerator()) {
        Invoke-Adb -Arguments @("shell", "setprop", $entry.Key, [string]$entry.Value) | Out-Null
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

function Capture-LaunchState {
    param(
        [string]$Label,
        [datetime]$LaunchStartedAt
    )
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
    $endFrame = @($log | Select-String -SimpleMatch "RUSTY_XR_MAKEPAD_OPENXR_END_FRAME").Count
    $visiblePanel = @($log | Select-String -SimpleMatch "visibleCameraProjectionReady=true").Count
    $xrCadence = @($log | Select-String -Pattern "RUSTY_XR_MAKEPAD_CADENCE.*xrUpdateRateHz=(?!0\\.00)").Count
    $loadingSignals = @($log | Select-String -Pattern "(?i)XrPermissionsFlow|preflight|loading").Count
    $brokerH264PrepareRequestCount = @($log | Select-String -SimpleMatch "phase=broker-h264-prepare-request status=sent").Count
    $brokerH264UnboundedHeaderCount = @($log | Select-String -SimpleMatch "packets=0 metadataBytes=").Count
    $brokerH264StreamHeaderMetadataCount = @($log | Select-String -SimpleMatch "phase=stream-header-metadata status=ok").Count
    $brokerH264PreparedCount = @($log | Select-String -SimpleMatch "phase=prepared status=ok").Count
    $brokerH264YuvTexturesReadyCount = @($log | Select-String -SimpleMatch "textureMode=cpu-yuv-decoded-broker-h264").Count
    $brokerH264TextureUpdateCount = @($log | Select-String -Pattern "phase=texture-updated status=ok.*cpuUploadPath=broker-h264-mediacodec-cpu-yuv").Count
    $brokerH264DecodeErrorCount = @($log | Select-String -Pattern "event=decode-error|Broker H[.]264 playback failed").Count
    $brokerH264ProgressLines = @($log | Select-String -SimpleMatch "Broker H.264 playback progress" | ForEach-Object { $_.Line })
    $brokerH264ProgressCount = @($brokerH264ProgressLines).Count
    $brokerH264CompleteProgressCount = @($brokerH264ProgressLines | Select-String -SimpleMatch "phase=complete").Count
    $brokerH264PacketsReadMax = 0
    $brokerH264InputQueuedMax = 0
    $brokerH264DecodedFrameMax = 0
    $brokerH264YuvFrameEmitMax = 0
    $brokerH264PacketReadRateHzMax = 0.0
    $brokerH264InputQueueRateHzMax = 0.0
    $brokerH264DecodedFrameRateHzMax = 0.0
    $brokerH264YuvFrameEmitRateHzMax = 0.0
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
    }
    $pairedCameraFrameCadenceCount = @($log | Select-String -SimpleMatch "pairedLeftRightCameraFrames=true").Count
    $alignedProjectionCadenceCount = @($log | Select-String -SimpleMatch "alignedProjection=true").Count
    $leftTextureUpdateMax = 0
    $rightTextureUpdateMax = 0
    foreach ($line in @($log | Select-String -SimpleMatch "RUSTY_XR_MAKEPAD_CADENCE" | ForEach-Object { $_.Line })) {
        if ($line -match "leftTextureUpdateCount=(\d+)") {
            $leftTextureUpdateMax = [Math]::Max($leftTextureUpdateMax, [int]$Matches[1])
        }
        if ($line -match "rightTextureUpdateCount=(\d+)") {
            $rightTextureUpdateMax = [Math]::Max($rightTextureUpdateMax, [int]$Matches[1])
        }
    }
    $projectionRuntimeManifestLines = @($log | Select-String -SimpleMatch "RUSTY_XR_PROJECTION_RUNTIME_MANIFEST" | ForEach-Object { $_.Line })
    $projectionRuntimeNumericTypeIssues = @(Get-ProjectionRuntimeNumericTypeIssues -LogLines $projectionRuntimeManifestLines)

    $state = [ordered]@{
        label = $Label
        launchedAt = $LaunchStartedAt.ToString("o")
        activeXrActivity = [bool]$activeXr
        activityHasExpectedPackage = [bool]$activityHasExpectedPackage
        windowHasExpectedPackage = [bool]$windowHasExpectedPackage
        activityHasExpectedXrActivity = [bool]$activityHasExpectedXrActivity
        windowHasExpectedXrActivity = [bool]$windowHasExpectedXrActivity
        openxrEndFrameCount = $endFrame
        visiblePanelMarkerCount = $visiblePanel
        nonzeroXrCadenceMarkerCount = $xrCadence
        loadingSignalCount = $loadingSignals
        processId = $processId
        appLineCount = @($appLog).Count
        appGpuFaultCount = @($appLog | Select-String -Pattern "(?i)page fault|gpu.*fault|kgsl|iommu|CP_SQE|faulting").Count
        gpuFaultCount = @($log | Select-String -Pattern "(?i)page fault|gpu.*fault|kgsl|iommu|CP_SQE|faulting").Count
        fatalCount = @($log | Select-String -Pattern "FATAL EXCEPTION|Fatal signal|signal 11|SIGSEGV|Abort message").Count
        hardwareBufferWarningCount = @($log | Select-String -Pattern "(?i)hardware.?buffer|AHardwareBuffer|GraphicBuffer\(w=4").Count
        projectionRuntimeManifestCount = $projectionRuntimeManifestLines.Count
        resolvedProjectionRuntimeEnabledMarkerCount = @($log | Select-String -SimpleMatch "RUSTY_XR_MAKEPAD_PROJECTION_RUNTIME" | Select-String -SimpleMatch "resolvedManifestConsumptionEnabled=true").Count
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
        brokerH264PreparedMarkerCount = $brokerH264PreparedCount
        brokerH264YuvTexturesReadyMarkerCount = $brokerH264YuvTexturesReadyCount
        brokerH264TextureUpdateMarkerCount = $brokerH264TextureUpdateCount
        brokerH264DecodeErrorMarkerCount = $brokerH264DecodeErrorCount
        brokerH264ProgressMarkerCount = $brokerH264ProgressCount
        brokerH264CompleteProgressMarkerCount = $brokerH264CompleteProgressCount
        brokerH264PacketsReadMax = $brokerH264PacketsReadMax
        brokerH264InputQueuedMax = $brokerH264InputQueuedMax
        brokerH264DecodedFrameMax = $brokerH264DecodedFrameMax
        brokerH264YuvFrameEmitMax = $brokerH264YuvFrameEmitMax
        brokerH264YuvCopyTimeMsMax = $brokerH264YuvCopyTimeMsMax
        brokerH264YuvCopyAvgMsMax = $brokerH264YuvCopyAvgMsMax
        brokerH264PacketReadRateHzMax = $brokerH264PacketReadRateHzMax
        brokerH264InputQueueRateHzMax = $brokerH264InputQueueRateHzMax
        brokerH264DecodedFrameRateHzMax = $brokerH264DecodedFrameRateHzMax
        brokerH264YuvFrameEmitRateHzMax = $brokerH264YuvFrameEmitRateHzMax
        brokerH264DecodedTextureReady = [bool]($brokerH264PreparedCount -gt 0 -and $brokerH264TextureUpdateCount -gt 0 -and $brokerH264DecodeErrorCount -eq 0)
        brokerH264LeftTextureUpdateMax = $leftTextureUpdateMax
        brokerH264RightTextureUpdateMax = $rightTextureUpdateMax
        pairedCameraFrameCadenceMarkerCount = $pairedCameraFrameCadenceCount
        alignedProjectionCadenceMarkerCount = $alignedProjectionCadenceCount
        brokerH264StereoProofMarkerCount = @($log | Select-String -SimpleMatch "cpuUploadPath=broker-h264-mediacodec-cpu-yuv").Count
        brokerH264SyntheticSourceMarkerCount = @($log | Select-String -SimpleMatch "sourceBindingMode=broker-h264-synthetic-stereo-stream").Count
        projectionHomographyReadyMarkerCount = @($log | Select-String -SimpleMatch "projectionHomographyReady=true").Count
        s71EyeCenteredMarkerCount = @($log | Select-String -SimpleMatch "s71EyeCenteredPanel=true").Count
        s71SharedPlaneParallaxRemovedMarkerCount = @($log | Select-String -SimpleMatch "s71SharedPlaneParallaxRemoved=true").Count
        staleS80PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s80-target-full-view-content-scale-panel-control").Count
        staleS79PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s79-target-source-eye-mapping-panel-control").Count
        staleS78PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s78-clipspace-surface-homography-panel-control").Count
        staleS77PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s77-rusty-xr-invalid-uv-fallback-panel-control").Count
        staleS76PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s76-direct-drawvars-homography-panel-control").Count
        staleS75PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s75-dynamic-homography-panel-control").Count
        staleS71PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s71-eye-centered-square-panel-control").Count
        staleS70PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s70-head-centered-aspect-panel-control").Count
        staleS69PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s69-source-eye-swap-panel-control").Count
        staleS68PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s68-active-eye-nonworld-panel-control").Count
        ready = [bool]($activeXr -and $processId -and $endFrame -gt 0 -and ($visiblePanel -gt 0 -or $xrCadence -gt 0))
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
            "--ez", "rustyxr.mediaProjection", "true",
            "--ei", "rustyxr.mediaProjectionPort", $MediaProjectionPort.ToString(),
            "--ei", "rustyxr.mediaProjectionWidth", $MediaProjectionWidth.ToString(),
            "--ei", "rustyxr.mediaProjectionHeight", $MediaProjectionHeight.ToString(),
            "--ei", "rustyxr.mediaProjectionDelayMs", $MediaProjectionDelayMs.ToString()
        )
    }
    Save-Adb -Arguments $launchArgs -Path (Join-Path $OutDir "$Label-start.txt")

    $deadline = (Get-Date).AddSeconds($StartupTimeoutSeconds)
    do {
        Start-Sleep -Seconds 3
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
        $remote = "/sdcard/rusty_xr_makepad_${Label}_$i.png"
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
        Start-Sleep -Seconds $FreshnessIntervalSeconds
    }
    return $hashes
}

function Wait-BrokerH264TextureReady {
    param([datetime]$LaunchStartedAt)
    if (-not ($UseBrokerH264Synthetic -or $UseBrokerH264Camera)) {
        return $null
    }

    $deadline = (Get-Date).AddSeconds($BrokerH264ReadyTimeoutSeconds)
    $state = $null
    do {
        Start-Sleep -Seconds 3
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
$effectiveProjectionRuntimeReadback = if ($ProjectionRuntimeReadback -eq "warn" -and $UseResolvedProjectionRuntime) { "required" } else { $ProjectionRuntimeReadback }

Invoke-Adb -Arguments @("devices") | Set-Content -Path (Join-Path $OutDir "adb-devices.txt") -Encoding UTF8
$projectionPropertyHygieneSummary = Invoke-RustyXrProjectionPropertyHygiene `
    -Adb "adb" `
    -Serial $Serial `
    -Mode $ProjectionPropertyHygiene `
    -OutputPath (Join-Path $OutDir "projection-property-hygiene.json")
Install-Apk
Grant-RuntimePermissions
Set-MakepadProjectionTargetProfile
Set-MakepadBrokerH264Profile
Save-Adb -Arguments @("shell", "dumpsys", "power") -Path (Join-Path $OutDir "power-before-launch.txt")
Save-Adb -Arguments @("shell", "getprop") -Path (Join-Path $OutDir "getprop-before-launch.txt")

$attempts = @()
if ($PreferDirectVrActivity) {
    $attempts += Start-ActivityAndProbe -Label "direct-vr-attempt-1" -Activity $XrActivity -ForceStopFirst -VrIntent
    if (-not $attempts[-1].ready) {
        $attempts += Start-ActivityAndProbe -Label "launcher-fallback-1" -Activity $LauncherActivity -LauncherIntent
    }
}
else {
    $attempts += Start-ActivityAndProbe -Label "launcher-attempt-1" -Activity $LauncherActivity -ForceStopFirst -LauncherIntent
    if (-not $attempts[-1].ready) {
        $attempts += Start-ActivityAndProbe -Label "launcher-attempt-2" -Activity $LauncherActivity -LauncherIntent
    }
    if (-not $attempts[-1].ready -and -not $SkipDirectXrFallback) {
        $attempts += Start-ActivityAndProbe -Label "direct-vr-fallback" -Activity $XrActivity -VrIntent
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
    $brokerReadyState = Wait-BrokerH264TextureReady -LaunchStartedAt $launchStartedAt
    if ($brokerReadyState) {
        $attempts += $brokerReadyState
    }
    $frames = Capture-FreshnessFrames -Label $finalLabel
    Start-Sleep -Seconds ([Math]::Max(0, $SampleSeconds - ($FreshnessFrames * $FreshnessIntervalSeconds)))
    $finalState = Capture-LaunchState -Label "$finalLabel-final" -LaunchStartedAt $launchStartedAt
    $attempts += $finalState
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
$projectionRuntimeReadbackSummary = Invoke-ProjectionRuntimeReadbackValidation -Attempts $attempts -Mode $effectiveProjectionRuntimeReadback
if ($effectiveProjectionRuntimeReadback -eq "required" -and $projectionRuntimeReadbackSummary.status -ne "ok") {
    $projectionRuntimeGateFailures += "projection runtime readback validation failed"
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

$summary = [ordered]@{
    schema = "rusty.xr.makepad-camera-device-gate.v1"
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
    brokerH264ProjectionGeometryProfile = $resolvedBrokerH264ProjectionGeometryProfile
    brokerH264SyntheticProjectionProfile = $resolvedBrokerH264SyntheticProjectionProfile
    projectionBorderPolicy = $ProjectionBorderPolicy
    nativePassthroughRequested = [bool]($EnableNativePassthrough -or $ProjectionBorderPolicy -eq "passthrough-underlay" -or $ProjectionAreaOpacity -lt 1.0 -or $ProjectionBorderOpacity -lt 1.0 -or $ProjectionAlphaMode -ne "fixed")
    projectionAreaOpacity = $ProjectionAreaOpacity
    projectionBorderOpacity = $ProjectionBorderOpacity
    projectionAlphaMode = $ProjectionAlphaMode
    projectionAlphaScale = $ProjectionAlphaScale
    projectionAlphaBias = $ProjectionAlphaBias
    useResolvedProjectionRuntime = [bool]$UseResolvedProjectionRuntime
    mediaProjection = [bool]$MediaProjection
    projectionPropertyHygiene = $projectionPropertyHygieneSummary
    projectionRuntimeReadbackMode = $effectiveProjectionRuntimeReadback
    projectionRuntimeReadback = $projectionRuntimeReadbackSummary
    mediaProjectionPort = $MediaProjectionPort
    mediaProjectionWidth = $MediaProjectionWidth
    mediaProjectionHeight = $MediaProjectionHeight
    mediaProjectionDelayMs = $MediaProjectionDelayMs
    processingLayer = $ProcessingLayer
    blurRadiusPx = $BlurRadiusPx
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
    uniqueFreshnessHashes = @($frames.sha256 | Sort-Object -Unique).Count
    freshnessFrames = $frames
}
$summary | ConvertTo-Json -Depth 7 | Set-Content -Path (Join-Path $OutDir "summary.json") -Encoding UTF8
$summary | ConvertTo-Json -Depth 7
if ($projectionRuntimeGateFailures.Count -gt 0) {
    throw "resolved projection runtime device gate failed: $($projectionRuntimeGateFailures -join '; ')"
}
