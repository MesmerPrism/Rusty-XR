param(
    [string]$Serial = "",
    [ValidateSet("controller", "polar")]
    [string]$Mode = "controller",
    [string]$Adb = "adb",
    [string]$BrokerHost = "127.0.0.1",
    [int]$BrokerPort = 8765,
    [double]$BaseScale = [double]::NaN,
    [double]$MaxScale = [double]::NaN,
    [double]$SmoothingAlpha = 0.30,
    [double]$MinQuality = 0.0,
    [int]$ConnectTimeoutMs = 250,
    [string]$OutDir = "",
    [switch]$SendCalibrationCommand,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"

function Format-InvariantDouble {
    param([double]$Value)
    return $Value.ToString("0.######", [Globalization.CultureInfo]::InvariantCulture)
}

function Assert-Scale {
    param(
        [string]$Name,
        [double]$Value
    )
    if ([double]::IsNaN($Value) -or [double]::IsInfinity($Value) -or $Value -lt 0.05 -or $Value -gt 1.50) {
        throw "$Name must be finite and within [0.05, 1.50]; got $Value"
    }
}

function Invoke-Adb {
    param([string[]]$Arguments)
    $adbArgs = @()
    if ($Serial) {
        $adbArgs += @("-s", $Serial)
    }
    $adbArgs += $Arguments
    & $Adb @adbArgs
    if ($LASTEXITCODE -ne 0) {
        throw "adb failed with exit code ${LASTEXITCODE}: $Adb $($adbArgs -join ' ')"
    }
}

function Set-AdbProperty {
    param(
        [string]$Name,
        [string]$Value
    )
    Invoke-Adb -Arguments @("shell", "setprop", $Name, $Value)
}

function Get-AdbProperty {
    param([string]$Name)
    $value = (Invoke-Adb -Arguments @("shell", "getprop", $Name)) -join ""
    return $value.Trim()
}

function Read-DoublePropertyOrDefault {
    param(
        [string]$Name,
        [double]$DefaultValue
    )
    $raw = Get-AdbProperty -Name $Name
    if ([string]::IsNullOrWhiteSpace($raw)) {
        return $DefaultValue
    }
    $parsed = 0.0
    if ([double]::TryParse($raw, [Globalization.NumberStyles]::Float, [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed) -and -not [double]::IsNaN($parsed) -and -not [double]::IsInfinity($parsed)) {
        return $parsed
    }
    return $DefaultValue
}

function Read-CurrentProjectionTargetScaleFromLogcat {
    $lines = Invoke-Adb -Arguments @("logcat", "-d")
    $projectionScaleMatches = @()
    foreach ($line in $lines) {
        $match = [regex]::Match($line, "projectionTargetScale=([0-9]+(?:\.[0-9]+)?)")
        if ($match.Success) {
            $parsed = 0.0
            if ([double]::TryParse($match.Groups[1].Value, [Globalization.NumberStyles]::Float, [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed) -and -not [double]::IsNaN($parsed) -and -not [double]::IsInfinity($parsed)) {
                $projectionScaleMatches += $parsed
            }
        }
    }
    if ($projectionScaleMatches.Count -eq 0) {
        return [double]::NaN
    }
    return $projectionScaleMatches[$projectionScaleMatches.Count - 1]
}

function Send-ManifoldCommand {
    param([object]$Command)
    $uri = [Uri]::new("ws://${BrokerHost}:${BrokerPort}/manifold/v1/events")
    $socket = [System.Net.WebSockets.ClientWebSocket]::new()
    $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromMilliseconds([Math]::Max($ConnectTimeoutMs, 250)))
    try {
        $socket.ConnectAsync($uri, $cts.Token).GetAwaiter().GetResult()
        $hello = [ordered]@{
            type = "hello"
            client_id = "host.makepad_pmb_scale_bridge"
            app_package = "rustyquest-makepad-camera-shell"
            role = "pmb_scale_bridge_cli"
        }
        foreach ($message in @($hello, $Command)) {
            $json = $message | ConvertTo-Json -Depth 12 -Compress
            $bytes = [Text.Encoding]::UTF8.GetBytes($json)
            $segment = [ArraySegment[byte]]::new($bytes)
            $socket.SendAsync($segment, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cts.Token).GetAwaiter().GetResult()
        }
    }
    finally {
        if ($socket.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
            $socket.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, "done", [Threading.CancellationToken]::None).GetAwaiter().GetResult()
        }
        $socket.Dispose()
        $cts.Dispose()
    }
}

if ($BrokerPort -lt 1 -or $BrokerPort -gt 65535) {
    throw "BrokerPort must be within [1, 65535]; got $BrokerPort"
}
if ($ConnectTimeoutMs -lt 50 -or $ConnectTimeoutMs -gt 5000) {
    throw "ConnectTimeoutMs must be within [50, 5000]; got $ConnectTimeoutMs"
}
if ($SmoothingAlpha -lt 0.0 -or $SmoothingAlpha -gt 1.0) {
    throw "SmoothingAlpha must be within [0, 1]; got $SmoothingAlpha"
}
if ($MinQuality -lt 0.0 -or $MinQuality -gt 1.0) {
    throw "MinQuality must be within [0, 1]; got $MinQuality"
}

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
if ([string]::IsNullOrWhiteSpace($OutDir)) {
    $OutDir = Join-Path $PSScriptRoot "..\..\..\target\makepad-pmb-scale-bridge\$timestamp"
}
$OutDir = [IO.Path]::GetFullPath($OutDir)
if (-not $PlanOnly) {
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
}

$baseScaleResolved = if ([double]::IsNaN($BaseScale)) {
    Read-DoublePropertyOrDefault -Name "debug.rustyquest.makepad.projection.target.scale" -DefaultValue 1.0
} else {
    $BaseScale
}
Assert-Scale -Name "BaseScale" -Value $baseScaleResolved

$currentScaleFromHeadset = Read-CurrentProjectionTargetScaleFromLogcat
$maxScaleResolved = if ([double]::IsNaN($MaxScale)) {
    if ([double]::IsNaN($currentScaleFromHeadset)) {
        $baseScaleResolved
    } else {
        $currentScaleFromHeadset
    }
} else {
    $MaxScale
}
Assert-Scale -Name "MaxScale" -Value $maxScaleResolved

$inputStream = if ($Mode -eq "controller") { "stream.motion.object_pose" } else { "bio:polar_acc" }
$posePublisherEnabled = if ($Mode -eq "controller") { "true" } else { "false" }

$properties = [ordered]@{
    "debug.rusty.manifold.broker.host" = $BrokerHost
    "debug.rusty.manifold.broker.port" = $BrokerPort.ToString([Globalization.CultureInfo]::InvariantCulture)
    "debug.rusty.manifold.breath.feedback.enabled" = "true"
    "debug.rusty.manifold.breath.feedback.stream" = "stream.breath.feedback_state"
    "debug.rusty.manifold.breath.feedback.receiver" = "app.makepad_camera_shell.breath_feedback"
    "debug.rusty.manifold.breath.feedback.connect.timeout.ms" = $ConnectTimeoutMs.ToString([Globalization.CultureInfo]::InvariantCulture)
    "debug.rusty.manifold.pose.publish.enabled" = $posePublisherEnabled
    "debug.rusty.manifold.pose.stream" = "stream.motion.object_pose"
    "debug.rusty.manifold.pose.source" = "provider.makepad.controller_pose"
    "debug.rusty.manifold.pose.controller" = "right"
    "debug.rusty.manifold.pose.kind" = "grip"
    "debug.rusty.manifold.pose.sample.hz" = "20"
    "debug.rusty.manifold.pose.connect.timeout.ms" = $ConnectTimeoutMs.ToString([Globalization.CultureInfo]::InvariantCulture)
    "debug.rustyquest.makepad.projection.target.breath.controls" = "scale"
    "debug.rustyquest.makepad.projection.target.breath.stream" = "stream.breath.feedback_state"
    "debug.rustyquest.makepad.projection.target.breath.min.scale" = (Format-InvariantDouble -Value $baseScaleResolved)
    "debug.rustyquest.makepad.projection.target.breath.max.scale" = (Format-InvariantDouble -Value $maxScaleResolved)
    "debug.rustyquest.makepad.projection.target.breath.smoothing.alpha" = (Format-InvariantDouble -Value $SmoothingAlpha)
    "debug.rustyquest.makepad.projection.target.breath.invert" = "false"
    "debug.rustyquest.makepad.projection.target.breath.min.quality" = (Format-InvariantDouble -Value $MinQuality)
}

$calibrationCommand = [ordered]@{
    type = "command"
    schema = "rusty.manifold.command.envelope.v1"
    request_id = "makepad-pmb-begin-calibration-$timestamp"
    command = "command.breath.begin_calibration"
    params = [ordered]@{
        schema = "rusty.manifold.projected_motion_breath.begin_calibration.v1"
        selected_source_preference = $Mode
        input_stream = $inputStream
        feedback_stream = "stream.breath.feedback_state"
        scale_at_volume0 = $baseScaleResolved
        scale_at_volume1 = $maxScaleResolved
        scale_endpoint_contract = "volume0=loaded-base-or-right-primary-reset,volume1=current-headset-target-scale"
    }
}

if (-not $PlanOnly) {
    foreach ($entry in $properties.GetEnumerator()) {
        Set-AdbProperty -Name $entry.Key -Value ([string]$entry.Value)
    }
}

$readback = @()
if (-not $PlanOnly) {
    foreach ($entry in $properties.GetEnumerator()) {
        $actual = Get-AdbProperty -Name $entry.Key
        $readback += [pscustomobject]@{
            property = $entry.Key
            expected = [string]$entry.Value
            actual = $actual
            matched = ($actual -eq [string]$entry.Value)
        }
    }
}

if ($SendCalibrationCommand -and -not $PlanOnly) {
    Send-ManifoldCommand -Command $calibrationCommand
}

$result = [ordered]@{
    schema = "rusty.quest.makepad.pmb_scale_bridge_setup.v1"
    mode = $Mode
    inputStream = $inputStream
    feedbackStream = "stream.breath.feedback_state"
    baseScale = $baseScaleResolved
    maxScale = $maxScaleResolved
    currentScaleFromHeadsetLogcat = $currentScaleFromHeadset
    smoothingAlpha = $SmoothingAlpha
    minQuality = $MinQuality
    brokerHost = $BrokerHost
    brokerPort = $BrokerPort
    properties = $properties
    readback = $readback
    calibrationCommand = $calibrationCommand
    calibrationCommandSent = [bool]($SendCalibrationCommand -and -not $PlanOnly)
    outDir = $OutDir
}

if (-not $PlanOnly) {
    $result | ConvertTo-Json -Depth 12 | Set-Content -Path (Join-Path $OutDir "pmb-scale-bridge-setup.json") -Encoding UTF8
    $calibrationCommand | ConvertTo-Json -Depth 12 | Set-Content -Path (Join-Path $OutDir "pmb-begin-calibration-command.json") -Encoding UTF8
}

$result | ConvertTo-Json -Depth 12
