param(
    [Parameter(Mandatory = $true)]
    [string]$Serial,

    [Parameter(Mandatory = $true)]
    [string]$Apk,

    [Parameter(Mandatory = $true)]
    [string]$PackageName,
    [Parameter(Mandatory = $true)]
    [string]$LauncherActivity,
    [Parameter(Mandatory = $true)]
    [string]$XrActivity,
    [string]$OutDir = "",
    [int]$StartupTimeoutSeconds = 30,
    [int]$SampleSeconds = 90,
    [int]$FreshnessFrames = 6,
    [int]$FreshnessIntervalSeconds = 1,
    [switch]$SkipInstall,
    [switch]$SkipDirectXrFallback
)

$ErrorActionPreference = "Stop"

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
    $activePattern = "$xrActivityPattern|$appPattern"
    $activeXr = @($activity | Select-String -Pattern $activePattern).Count -gt 0 -and
        @($window | Select-String -Pattern $activePattern).Count -gt 0
    $endFrame = @($log | Select-String -SimpleMatch "RUSTY_XR_MAKEPAD_OPENXR_END_FRAME").Count
    $visiblePanel = @($log | Select-String -SimpleMatch "visibleCameraProjectionReady=true").Count
    $xrCadence = @($log | Select-String -Pattern "RUSTY_XR_MAKEPAD_CADENCE.*xrUpdateRateHz=(?!0\\.00)").Count
    $loadingSignals = @($log | Select-String -Pattern "(?i)XrPermissionsFlow|preflight|loading").Count

    $state = [ordered]@{
        label = $Label
        launchedAt = $LaunchStartedAt.ToString("o")
        activeXrActivity = [bool]$activeXr
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
        s69bMarkerCount = @($log | Select-String -SimpleMatch "s69bHorizontalMirrorFix=true").Count
        s70SquareAspectMarkerCount = @($log | Select-String -SimpleMatch "s70SquareAspectFix=true").Count
        s72HeadCenteredSquareRestoredMarkerCount = @($log | Select-String -SimpleMatch "s72HeadCenteredSquareRestored=true").Count
        s72MetadataUvBaselineCorrectionMarkerCount = @($log | Select-String -SimpleMatch "s72MetadataUvBaselineCorrection=true").Count
        s73ScalarHomographyBindingMarkerCount = @($log | Select-String -SimpleMatch "s73ScalarHomographyBinding=true").Count
        s74LiteralHomographyRowsMarkerCount = @($log | Select-String -SimpleMatch "s74LiteralHomographyRows=true").Count
        s75DynamicHomographyBindingMarkerCount = @($log | Select-String -SimpleMatch "s75DynamicHomographyBinding=true").Count
        s76DirectDrawVarsHomographyMarkerCount = @($log | Select-String -SimpleMatch "s76DirectDrawVarsHomography=true").Count
        s77RustyXrInvalidUvFallbackMarkerCount = @($log | Select-String -SimpleMatch "s77RustyXrInvalidUvFallback=true").Count
        s78ClipSpaceSurfaceHomographyMarkerCount = @($log | Select-String -SimpleMatch "s78ClipSpaceSurfaceHomography=true").Count
        s79TargetSourceEyeMappingMarkerCount = @($log | Select-String -SimpleMatch "s79TargetSourceEyeMapping=true").Count
        s80FullViewContentUvScaleMarkerCount = @($log | Select-String -SimpleMatch "s80FullViewContentUvScale=true").Count
        s81DynamicScreenSurfaceUvMarkerCount = @($log | Select-String -SimpleMatch "s81DynamicScreenSurfaceUv=true").Count
        s82CollapsedScreenToCameraHomographyMarkerCount = @($log | Select-String -SimpleMatch "s82CollapsedScreenToCameraHomography=true").Count
        s83DrawPassProjectionInverseHomographyMarkerCount = @($log | Select-String -SimpleMatch "s83DrawPassProjectionInverseHomography=true").Count
        s84ProjectionInverseNearFarFallbackMarkerCount = @($log | Select-String -SimpleMatch "s84ProjectionInverseNearFarFallback=true").Count
        s85ForcedScreenToCameraFallbackMarkerCount = @($log | Select-String -SimpleMatch "s85ForcedScreenToCameraFallback=true").Count
        s87RuntimeXrViewHomographyMarkerCount = @($log | Select-String -SimpleMatch "s87RuntimeXrViewHomography=true").Count
        s88TargetFastInvalidFallbackMarkerCount = @($log | Select-String -SimpleMatch "s88TargetFastInvalidFallback=true").Count
        s89SingleQuadTargetScreenUvMarkerCount = @($log | Select-String -SimpleMatch "s89SingleQuadTargetScreenUv=true").Count
        s90CameraIdSourceBindingMarkerCount = @($log | Select-String -SimpleMatch "s90CameraIdSourceBinding=true").Count
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
        staleS88PathMarkerCount = @($log | Select-String -SimpleMatch "makepad-s88-target-fast-invalid-fallback").Count
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
        ready = [bool]($activeXr -and $endFrame -gt 0 -and ($visiblePanel -gt 0 -or $xrCadence -gt 0))
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
        [switch]$LauncherIntent
    )
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
    $launchArgs += @("-n", $component)
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
        Invoke-Adb -Arguments @("pull", $remote, $local) | Out-Null
        Invoke-Adb -Arguments @("shell", "rm", $remote) | Out-Null
        $hashes += [ordered]@{
            file = $local
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $local).Hash
            length = (Get-Item -LiteralPath $local).Length
        }
        Start-Sleep -Seconds $FreshnessIntervalSeconds
    }
    return $hashes
}

if (-not $OutDir) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutDir = Join-Path (Get-Location) "artifacts/makepad-q2q-device-gate-$stamp"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Invoke-Adb -Arguments @("devices") | Set-Content -Path (Join-Path $OutDir "adb-devices.txt") -Encoding UTF8
Install-Apk
Grant-RuntimePermissions
Save-Adb -Arguments @("shell", "dumpsys", "power") -Path (Join-Path $OutDir "power-before-launch.txt")
Save-Adb -Arguments @("shell", "getprop") -Path (Join-Path $OutDir "getprop-before-launch.txt")

$attempts = @()
$attempts += Start-ActivityAndProbe -Label "launcher-attempt-1" -Activity $LauncherActivity -ForceStopFirst -LauncherIntent
if (-not $attempts[-1].ready) {
    $attempts += Start-ActivityAndProbe -Label "launcher-attempt-2" -Activity $LauncherActivity -LauncherIntent
}
if (-not $attempts[-1].ready -and -not $SkipDirectXrFallback) {
    $attempts += Start-ActivityAndProbe -Label "direct-xr-fallback" -Activity $XrActivity
}

$finalLabel = $attempts[-1].label
if ($attempts[-1].ready) {
    $frames = Capture-FreshnessFrames -Label $finalLabel
    Start-Sleep -Seconds ([Math]::Max(0, $SampleSeconds - ($FreshnessFrames * $FreshnessIntervalSeconds)))
    $finalState = Capture-LaunchState -Label "$finalLabel-final" -LaunchStartedAt (Get-Date)
    $attempts += $finalState
} else {
    $frames = @()
}

$readyAttempt = $attempts |
    Where-Object { $_.label -notlike "*-final" -and $_.ready } |
    Select-Object -First 1

$summary = [ordered]@{
    schema = "rusty.xr.makepad-q2q-device-gate.v1"
    capturedAt = (Get-Date).ToString("o")
    serial = $Serial
    packageName = $PackageName
    apk = $Apk
    launchReady = [bool]$readyAttempt
    recoveredBy = if ($readyAttempt) { $readyAttempt.label } else { "none" }
    attempts = $attempts
    uniqueFreshnessHashes = @($frames.sha256 | Sort-Object -Unique).Count
    freshnessFrames = $frames
}
$summary | ConvertTo-Json -Depth 7 | Set-Content -Path (Join-Path $OutDir "summary.json") -Encoding UTF8
$summary | ConvertTo-Json -Depth 7
