[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory = $true)]
    [string]$Serial,

    [Parameter(Mandatory = $true)]
    [string]$Apk,

    [string]$PackageName = "io.github.mesmerprism.rustyxr.makepad.camera",
    [string]$LauncherActivity = ("." + "Makepad" + "App"),
    [string]$XrActivity = ("." + "Makepad" + "App" + "Xr"),
    [string]$OutDir = "",

    [ValidateSet("direct", "broker")]
    [string[]]$Lane = @("direct"),

    [double[]]$AreaOpacity = @(0.0, 0.5, 1.0),
    [int]$StartupTimeoutSeconds = 40,
    [int]$SampleSeconds = 14,
    [int]$FreshnessFrames = 4,
    [int]$FreshnessIntervalSeconds = 1,
    [int]$BrokerH264ReadyTimeoutSeconds = 45,
    [switch]$SkipInstall,
    [switch]$RestartBrokerBeforeBrokerRows,
    [string]$Adb = $env:RUSTY_XR_ADB,

    [string]$BrokerH264LeftCameraId = "50",
    [string]$BrokerH264RightCameraId = "51",
    [int]$BrokerH264CaptureMs = 0,
    [int]$BrokerH264MaxPackets = 0,
    [int]$BrokerH264FrameRateHz = 50,
    [string]$BrokerPackageName = "com.example.rustyxr.broker",
    [string]$BrokerActivityName = ".BrokerStartActivity",
    [int]$BrokerRestartSettleSeconds = 8
)

$ErrorActionPreference = "Stop"

function Format-InvariantDouble {
    param([double]$Value)
    return $Value.ToString("0.######", [System.Globalization.CultureInfo]::InvariantCulture)
}

function Format-OpacityToken {
    param([double]$Value)
    return (Format-InvariantDouble -Value $Value).Replace(".", "p")
}

function Invoke-Adb {
    param([string[]]$Arguments)
    if (-not $Adb) {
        $script:Adb = "adb"
    }
    & $Adb -s $Serial @Arguments
}

function Resolve-AndroidComponent {
    param(
        [string]$PackageName,
        [string]$ActivityName
    )
    if ($ActivityName.Contains("/")) {
        return $ActivityName
    }
    if ($ActivityName.StartsWith(".")) {
        return "$PackageName/$ActivityName"
    }
    return "$PackageName/$ActivityName"
}

function Restart-BrokerForRow {
    param([string]$RowOut)

    if (-not $RestartBrokerBeforeBrokerRows) {
        return $null
    }

    $restartDir = Join-Path $RowOut "broker-restart"
    New-Item -ItemType Directory -Force -Path $restartDir | Out-Null
    $component = Resolve-AndroidComponent -PackageName $BrokerPackageName -ActivityName $BrokerActivityName

    Invoke-Adb -Arguments @("shell", "pidof", $BrokerPackageName) 2>&1 |
        Set-Content -Path (Join-Path $restartDir "before-pid.txt") -Encoding UTF8
    Invoke-Adb -Arguments @("shell", "am", "force-stop", $BrokerPackageName) 2>&1 |
        Set-Content -Path (Join-Path $restartDir "force-stop.txt") -Encoding UTF8
    Start-Sleep -Seconds 2
    Invoke-Adb -Arguments @("shell", "am", "start", "-n", $component) 2>&1 |
        Set-Content -Path (Join-Path $restartDir "start.txt") -Encoding UTF8
    Start-Sleep -Seconds ([Math]::Max(1, $BrokerRestartSettleSeconds))
    Invoke-Adb -Arguments @("shell", "pidof", $BrokerPackageName) 2>&1 |
        Set-Content -Path (Join-Path $restartDir "after-pid.txt") -Encoding UTF8

    return [ordered]@{
        dir = $restartDir
        packageName = $BrokerPackageName
        activityName = $BrokerActivityName
        component = $component
        settleSeconds = $BrokerRestartSettleSeconds
    }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$makepadGate = Join-Path $repoRoot "examples\makepad-camera-shell\tools\Invoke-MakepadCameraDeviceGate.ps1"
if (-not (Test-Path -LiteralPath $makepadGate)) {
    throw "Makepad device gate not found: $makepadGate"
}

if (-not $OutDir) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutDir = Join-Path (Get-Location) "artifacts\makepad-opacity-ladder-$stamp"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$records = @()
foreach ($laneName in $Lane) {
    foreach ($opacity in $AreaOpacity) {
        $opacityText = Format-InvariantDouble -Value $opacity
        $runName = "{0}-opacity-{1}" -f $laneName, (Format-OpacityToken -Value $opacity)
        $runOut = Join-Path $OutDir $runName
        New-Item -ItemType Directory -Force -Path $runOut | Out-Null
        $brokerRestart = if ($laneName -eq "broker") {
            Restart-BrokerForRow -RowOut $runOut
        } else {
            $null
        }

        $args = @(
            "-NoProfile", "-ExecutionPolicy", "Bypass",
            "-File", $makepadGate,
            "-Serial", $Serial,
            "-Apk", $Apk,
            "-PackageName", $PackageName,
            "-LauncherActivity", $LauncherActivity,
            "-XrActivity", $XrActivity,
            "-OutDir", $runOut,
            "-PreferDirectVrActivity",
            "-ProjectionBorderPolicy", "passthrough-underlay",
            "-ProjectionAreaOpacity", $opacityText,
            "-ProjectionBorderOpacity", "0",
            "-ProcessingLayer", "raw",
            "-ProjectionScale", "1",
            "-XrRenderScale", "1",
            "-ProjectionAreaScaleX", "1",
            "-ProjectionAreaScaleY", "1",
            "-ProjectionAreaRadiusXUv", "0.5",
            "-ProjectionAreaRadiusYUv", "0.5",
            "-ProjectionAreaCornerRadiusUv", "0",
            "-StartupTimeoutSeconds", $StartupTimeoutSeconds,
            "-SampleSeconds", $SampleSeconds,
            "-FreshnessFrames", $FreshnessFrames,
            "-FreshnessIntervalSeconds", $FreshnessIntervalSeconds,
            "-BrokerH264ReadyTimeoutSeconds", $BrokerH264ReadyTimeoutSeconds
        )
        if ($SkipInstall) {
            $args += "-SkipInstall"
        }
        if ($laneName -eq "broker") {
            $args += @(
                "-UseBrokerH264Camera",
                "-BrokerH264CaptureMs", $BrokerH264CaptureMs,
                "-BrokerH264MaxPackets", $BrokerH264MaxPackets,
                "-BrokerH264FrameRateHz", $BrokerH264FrameRateHz,
                "-BrokerH264LeftCameraId", $BrokerH264LeftCameraId,
                "-BrokerH264RightCameraId", $BrokerH264RightCameraId
            )
        }

        $consolePath = Join-Path $runOut "run-console.txt"
        $previousErrorActionPreference = $ErrorActionPreference
        $previousNativeErrorPreference = if (Test-Path variable:PSNativeCommandUseErrorActionPreference) {
            $PSNativeCommandUseErrorActionPreference
        } else {
            $null
        }
        try {
            $ErrorActionPreference = "Continue"
            if (Test-Path variable:PSNativeCommandUseErrorActionPreference) {
                $PSNativeCommandUseErrorActionPreference = $false
            }
            $consoleOutput = & powershell @args 2>&1
            $exitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $previousErrorActionPreference
            if (Test-Path variable:PSNativeCommandUseErrorActionPreference) {
                $PSNativeCommandUseErrorActionPreference = $previousNativeErrorPreference
            }
        }
        $consoleOutput | Set-Content -Path $consolePath -Encoding UTF8

        $summaryPath = Join-Path $runOut "summary.json"
        $summary = if (Test-Path -LiteralPath $summaryPath) {
            Get-Content -LiteralPath $summaryPath -Raw | ConvertFrom-Json
        } else {
            $null
        }
        $finalAttempt = if ($summary -and $summary.attempts -and $summary.attempts.Count -gt 0) {
            $summary.attempts[-1]
        } else {
            $null
        }
        $brokerTextureReady = if ($finalAttempt) {
            [bool]($finalAttempt.brokerH264TextureUpdateMarkerCount -gt 0 -or $finalAttempt.brokerH264DecodedTextureReady)
        } else {
            $false
        }
        $status = if ($exitCode -ne 0 -or -not $summary) {
            "failed"
        } elseif ($laneName -eq "broker" -and -not $brokerTextureReady) {
            "blocked-broker-texture-not-ready"
        } else {
            "manual-visual-review-required"
        }

        $records += [ordered]@{
            lane = $laneName
            projectionAreaOpacity = $opacity
            projectionBorderOpacity = 0.0
            status = $status
            runDir = $runOut
            summary = $summaryPath
            console = $consolePath
            exitCode = $exitCode
            launchReady = if ($summary) { [bool]$summary.launchReady } else { $false }
            nativePassthroughRequested = if ($summary) { [bool]$summary.nativePassthroughRequested } else { $false }
            uniqueFreshnessHashes = if ($summary) { [int]$summary.uniqueFreshnessHashes } else { 0 }
            freshnessFrames = if ($summary) { @($summary.freshnessFrames.file) } else { @() }
            brokerH264TextureUpdateMarkerCount = if ($finalAttempt) { [int]$finalAttempt.brokerH264TextureUpdateMarkerCount } else { 0 }
            brokerH264DecodedTextureReady = if ($finalAttempt) { [bool]$finalAttempt.brokerH264DecodedTextureReady } else { $false }
            brokerRestart = $brokerRestart
            expectedVisual = if ($opacity -le 0.0001) {
                "native passthrough only; no custom camera RGB"
            } elseif ($opacity -ge 0.9999) {
                "custom camera projection"
            } else {
                "intentional native/custom blend"
            }
        }
    }
}

$summaryOut = [ordered]@{
    schema = "rusty.xr.makepad-opacity-ladder-gate.v1"
    capturedAt = (Get-Date).ToString("o")
    serial = $Serial
    apk = $Apk
    packageName = $PackageName
    lanes = $Lane
    opacityValues = $AreaOpacity
    restartBrokerBeforeBrokerRows = [bool]$RestartBrokerBeforeBrokerRows
    outDir = $OutDir
    records = $records
}
$summaryPath = Join-Path $OutDir "makepad-opacity-ladder-summary.json"
$summaryOut | ConvertTo-Json -Depth 7 | Set-Content -Path $summaryPath -Encoding UTF8
$summaryOut | ConvertTo-Json -Depth 7
