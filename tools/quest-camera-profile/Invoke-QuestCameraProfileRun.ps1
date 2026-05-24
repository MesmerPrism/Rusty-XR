param(
    [string]$Serial = "",
    [string]$Adb = "adb",
    [string]$Npx = "npx",
    [string]$HzdbNpxPackage = "@meta-quest/hzdb",
    [string]$Catalog = "examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json",
    [string]$AppId = "rusty-xr-quest-composite-layer",
    [string]$DeviceProfile = "xr-composite-smoke-test",
    [string]$RuntimeProfile = "camera-stereo-gpu-composite-performance-065",
    [string]$CameraPipelinePreset = "",
    [ValidateSet("", "display-screen-homography", "quad-surface")]
    [string]$CameraProjectionMode = "",
    [string]$RunRoot = "artifacts\quest-camera-profile-runs",
    [int]$WarmupSeconds = 14,
    [string[]]$Override = @(),
    [ValidateSet("com.oculus.intent.category.VR", "android.intent.category.LAUNCHER")]
    [string]$LaunchCategory = "com.oculus.intent.category.VR",
    [string]$LaunchActivity = "",
    [switch]$Install,
    [string]$Apk = "",
    [switch]$UseProximityHold,
    [switch]$SkipProximityHold,
    [int]$ProximityHoldDurationMs = 600000,
    [switch]$CaptureHzdbScreencap,
    [switch]$CaptureMetacam,
    [int]$FreshnessFrames = 0,
    [int]$FreshnessIntervalMs = 1000,
    [switch]$FailOnPowerStateDrift,
    [int]$LogcatLines = 12000,
    [ValidateSet("fail", "clear", "ignore")]
    [string]$ProjectionPropertyHygiene = "fail",
    [ValidateSet("skip", "warn", "required")]
    [string]$ProjectionRuntimeReadback = "warn",
    [string]$ProjectionRuntimeReadbackValidator = "",
    [string]$Validator = ""
)

$ErrorActionPreference = "Stop"

if ($UseProximityHold -and $SkipProximityHold) {
    throw "-UseProximityHold and -SkipProximityHold cannot be used together."
}
$proximityHoldRequested = [bool]$UseProximityHold

if (-not $Validator) {
    $Validator = Join-Path $PSScriptRoot "Validate-QuestCameraRun.py"
}
if (-not $ProjectionRuntimeReadbackValidator) {
    $ProjectionRuntimeReadbackValidator = Join-Path $PSScriptRoot "Validate-ProjectionRuntimeReadback.py"
}
$projectionPropertyHygieneHelper = Join-Path $PSScriptRoot "ProjectionPropertyHygiene.ps1"
. $projectionPropertyHygieneHelper

function Get-ProjectionRuntimeExpectedBackend {
    if ($AppId -match "gl-openxr-video-stack" -or $Catalog -match "quest-gl-openxr-video-stack") {
        return "oes"
    }
    if ($AppId -match "composite-layer" -or $Catalog -match "quest-composite-layer") {
        return "hwb"
    }
    return "any"
}

function Resolve-InputPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))
}

function Convert-ToExtendedWindowsPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "Cannot normalize an empty path."
    }
    try {
        $fullPath = [System.IO.Path]::GetFullPath($Path)
    }
    catch {
        $preview = $Path.Replace("`0", "<NUL>")
        if ($preview.Length -gt 240) {
            $preview = $preview.Substring(0, 240) + "..."
        }
        throw "Failed to normalize path (length=$($Path.Length), preview='$preview'): $($_.Exception.Message)"
    }
    if ($env:OS -eq "Windows_NT" -and -not $fullPath.StartsWith("\\?\")) {
        return "\\?\$fullPath"
    }
    return $fullPath
}

function Get-FileSha256Hex {
    param([string]$Path)
    $stream = [System.IO.File]::OpenRead((Convert-ToExtendedWindowsPath -Path $Path))
    try {
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            $bytes = $sha.ComputeHash($stream)
            return [BitConverter]::ToString($bytes).Replace("-", "")
        }
        finally {
            $sha.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Get-FileByteLength {
    param([string]$Path)
    return ([System.IO.FileInfo]::new((Convert-ToExtendedWindowsPath -Path $Path))).Length
}

function Write-Utf8TextFile {
    param(
        [string]$Path,
        [object]$Value
    )
    $directory = Split-Path -Path $Path -Parent
    if ($directory) {
        [System.IO.Directory]::CreateDirectory((Convert-ToExtendedWindowsPath -Path $directory)) | Out-Null
    }
    $text = if ($Value -is [array]) { $Value -join [Environment]::NewLine } else { [string]$Value }
    $encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText((Convert-ToExtendedWindowsPath -Path $Path), $text, $encoding)
}

function Get-AdbArguments {
    param([string[]]$Arguments)
    if ($Serial) {
        return @("-s", $Serial) + $Arguments
    }
    return $Arguments
}

function Invoke-Adb {
    param([string[]]$Arguments)
    $adbArguments = @(Get-AdbArguments -Arguments $Arguments)
    & $Adb @adbArguments
}

function Save-AdbTextCapture {
    param(
        [string[]]$Arguments,
        [string]$OutputPath
    )
    Write-Utf8TextFile -Path $OutputPath -Value ((Invoke-Adb -Arguments $Arguments) -join [Environment]::NewLine)
}

function Get-ProfileRunLogcatPath {
    param(
        [string]$Dir,
        [string]$Label
    )

    return Join-Path $Dir "logcat-window.txt"
}

function Format-ProcessArguments {
    param([string[]]$ArgumentList)

    return @($ArgumentList | ForEach-Object {
        if ($_ -match '\s') {
            '"' + ($_ -replace '"', '\"') + '"'
        }
        else {
            $_
        }
    }) -join " "
}

function Start-AdbLogcatWindowCapture {
    param(
        [string]$Dir,
        [string]$Label
    )

    $logcatPath = Get-ProfileRunLogcatPath -Dir $Dir -Label $Label
    $commandPath = Join-Path $Dir "$Label-logcat-window-command.txt"
    $stderrPath = Join-Path $Dir "$Label-logcat-window-stderr.txt"
    $resolvedAdb = Resolve-ProcessFileName -FileName $Adb
    $clearArguments = @(Get-AdbArguments -Arguments @("logcat", "-c"))
    $streamArguments = @(Get-AdbArguments -Arguments @("logcat", "-v", "threadtime"))
    Write-Utf8TextFile -Path $commandPath -Value @(
        "$resolvedAdb $($clearArguments -join ' ')",
        "$resolvedAdb $($streamArguments -join ' ') > $logcatPath 2> $stderrPath"
    )

    $clearOutput = @()
    $clearExitCode = 0
    try {
        $previousErrorActionPreference = $ErrorActionPreference
        try {
            $ErrorActionPreference = "Continue"
            $clearOutput = @(Invoke-Adb -Arguments @("logcat", "-c") 2>&1 | ForEach-Object { [string]$_ })
            $clearExitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
    }
    catch {
        $clearExitCode = 1
        $clearOutput = @($_.Exception.Message)
    }

    $process = $null
    $startError = ""
    if (Test-Path -LiteralPath $logcatPath) {
        Remove-Item -LiteralPath $logcatPath -Force
    }
    if (Test-Path -LiteralPath $stderrPath) {
        Remove-Item -LiteralPath $stderrPath -Force
    }
    try {
        $streamArgumentText = Format-ProcessArguments -ArgumentList $streamArguments
        $process = Start-Process `
            -FilePath $resolvedAdb `
            -ArgumentList $streamArgumentText `
            -RedirectStandardOutput $logcatPath `
            -RedirectStandardError $stderrPath `
            -WindowStyle Hidden `
            -PassThru
        Start-Sleep -Milliseconds 250
    }
    catch {
        $startError = $_.Exception.Message
    }

    return [ordered]@{
        schemaVersion = "rusty.xr.quest-camera-logcat-window.v1"
        mode = "streaming-window"
        path = $logcatPath
        stderrPath = $stderrPath
        commandPath = $commandPath
        clearExitCode = $clearExitCode
        clearOutput = @($clearOutput)
        startedAt = (Get-Date).ToString("o")
        stoppedAt = ""
        processId = if ($process) { $process.Id } else { $null }
        startError = $startError
        exitCode = $null
        bytes = 0
        stderrBytes = 0
        stopError = ""
        process = $process
    }
}

function Stop-AdbLogcatWindowCapture {
    param([object]$Capture)

    if (-not $Capture) {
        return $null
    }

    try {
        $process = $Capture.process
        if ($process -and -not $process.HasExited) {
            try {
                $process.Kill()
            }
            catch {
                $Capture.stopError = $_.Exception.Message
            }
            try {
                $process.WaitForExit(5000) | Out-Null
            }
            catch {
            }
        }
        if ($process -and $process.HasExited) {
            $Capture.exitCode = $process.ExitCode
        }
    }
    catch {
        $Capture.stopError = $_.Exception.Message
    }
    finally {
        if ($Capture.Contains("process")) {
            $Capture.Remove("process")
        }
        $Capture.stoppedAt = (Get-Date).ToString("o")
        if (Test-Path -LiteralPath $Capture.path) {
            $Capture.bytes = (Get-Item -LiteralPath $Capture.path).Length
        }
        if ($Capture.stderrPath -and (Test-Path -LiteralPath $Capture.stderrPath)) {
            $Capture.stderrBytes = (Get-Item -LiteralPath $Capture.stderrPath).Length
        }
    }

    return $Capture
}

function Test-TruthyLaunchValue {
    param([string]$Value)
    if ([string]::IsNullOrWhiteSpace($Value)) {
        return $false
    }
    $normalized = $Value.Trim().ToLowerInvariant()
    return $normalized -in @("1", "true", "yes", "on")
}

function Grant-MediaProjectionAppOp {
    param(
        [string]$PackageName,
        [string]$Dir
    )
    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add("adb shell appops set $PackageName PROJECT_MEDIA allow")
    $setOutput = @()
    foreach ($line in (Invoke-Adb -Arguments @("shell", "appops", "set", $PackageName, "PROJECT_MEDIA", "allow") 2>&1)) {
        $setOutput += [string]$line
        $lines.Add([string]$line)
    }
    $setExitCode = $LASTEXITCODE
    $lines.Add("adb shell appops get $PackageName PROJECT_MEDIA")
    $getOutput = @()
    foreach ($line in (Invoke-Adb -Arguments @("shell", "appops", "get", $PackageName, "PROJECT_MEDIA") 2>&1)) {
        $getOutput += [string]$line
        $lines.Add([string]$line)
    }
    $getExitCode = $LASTEXITCODE
    Write-Utf8TextFile -Path (Join-Path $Dir "mediaprojection-appops.txt") -Value $lines.ToArray()
    if ($setExitCode -ne 0 -or $getExitCode -ne 0 -or (($getOutput -join "`n") -notmatch "PROJECT_MEDIA:\s*allow")) {
        throw "MediaProjection PROJECT_MEDIA app-op pregrant failed or did not read back as allow; see mediaprojection-appops.txt"
    }
}

function Save-OptionalRunAsFileCapture {
    param(
        [string]$Package,
        [string]$RemotePath,
        [string]$OutputPath
    )

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $probeOutput = @(Invoke-Adb -Arguments @("shell", "run-as", $Package, "ls", $RemotePath) 2>&1 | ForEach-Object { [string]$_ })
        $probeExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($probeExitCode -ne 0) {
        Write-Utf8TextFile -Path "$OutputPath.missing.txt" -Value @(
            "Optional run-as file was not captured.",
            "package=$Package",
            "remotePath=$RemotePath",
            "probeExitCode=$probeExitCode",
            "probeOutput:",
            $probeOutput
        )
        return
    }

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $captureOutput = @(Invoke-Adb -Arguments @("shell", "run-as", $Package, "cat", $RemotePath) 2>&1 | ForEach-Object { [string]$_ })
        $captureExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($captureExitCode -ne 0) {
        Write-Utf8TextFile -Path "$OutputPath.error.txt" -Value @(
            "Optional run-as file probe succeeded, but capture failed.",
            "package=$Package",
            "remotePath=$RemotePath",
            "captureExitCode=$captureExitCode",
            "captureOutput:",
            $captureOutput
        )
        return
    }

    Write-Utf8TextFile -Path $OutputPath -Value $captureOutput
}

function Resolve-ProcessFileName {
    param([string]$FileName)
    if ([System.IO.Path]::IsPathRooted($FileName) -or (Split-Path -Path $FileName -Parent)) {
        return $FileName
    }

    $candidates = @()
    if ($env:OS -eq "Windows_NT" -and -not [System.IO.Path]::HasExtension($FileName)) {
        $candidates += @("$FileName.cmd", "$FileName.exe", "$FileName.bat")
    }
    $candidates += $FileName

    foreach ($candidate in $candidates) {
        $command = Get-Command $candidate -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($command -and $command.Path) {
            return $command.Path
        }
    }

    return $FileName
}

function Invoke-ProcessCapture {
    param(
        [string]$FileName,
        [string[]]$ArgumentList,
        [string]$OutputPath,
        [int]$TimeoutSeconds = 120
    )

    $quotedArguments = $ArgumentList | ForEach-Object {
        if ($_ -match '\s') {
            '"' + ($_ -replace '"', '\"') + '"'
        }
        else {
            $_
        }
    }

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $resolvedFileName = Resolve-ProcessFileName -FileName $FileName
    $startInfo.FileName = $resolvedFileName
    $startInfo.Arguments = $quotedArguments -join " "
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    if (-not $process.Start()) {
        throw "Failed to start $FileName"
    }

    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        try {
            $process.Kill($true)
        }
        catch {
        }
        throw "$FileName timed out after $TimeoutSeconds seconds"
    }

    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $capturedLines = @(
        "fileName=$FileName"
        "resolvedFileName=$resolvedFileName"
        "exitCode=$($process.ExitCode)"
        ""
        "[stdout]"
        $stdout
        ""
        "[stderr]"
        $stderr
    )
    Write-Utf8TextFile -Path $OutputPath -Value $capturedLines

    if ($process.ExitCode -ne 0) {
        throw "$FileName failed with exit code $($process.ExitCode); see $OutputPath"
    }
}

function Invoke-AdbBinaryCapture {
    param(
        [string[]]$Arguments,
        [string]$OutputPath,
        [int]$TimeoutSeconds = 30,
        [int]$RetryCount = 2
    )

    $lastError = ""
    for ($attempt = 1; $attempt -le ($RetryCount + 1); $attempt++) {
        $argumentList = Get-AdbArguments -Arguments $Arguments
        $quotedArguments = $argumentList | ForEach-Object {
            if ($_ -match '\s') {
                '"' + ($_ -replace '"', '\"') + '"'
            }
            else {
                $_
            }
        }

        $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
        $startInfo.FileName = Resolve-ProcessFileName -FileName $Adb
        $startInfo.Arguments = $quotedArguments -join " "
        $startInfo.UseShellExecute = $false
        $startInfo.CreateNoWindow = $true
        $startInfo.RedirectStandardOutput = $true
        $startInfo.RedirectStandardError = $true

        $process = [System.Diagnostics.Process]::new()
        $process.StartInfo = $startInfo
        if (-not $process.Start()) {
            throw "Failed to start adb for binary capture."
        }

        $outputDirectory = Split-Path -Path $OutputPath -Parent
        if ($outputDirectory) {
            [System.IO.Directory]::CreateDirectory((Convert-ToExtendedWindowsPath -Path $outputDirectory)) | Out-Null
        }
        $fileStream = [System.IO.File]::Create((Convert-ToExtendedWindowsPath -Path $OutputPath))
        try {
            $process.StandardOutput.BaseStream.CopyTo($fileStream)
        }
        finally {
            $fileStream.Dispose()
        }

        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            try {
                $process.Kill($true)
            }
            catch {
            }
            $lastError = "adb binary capture timed out after $TimeoutSeconds seconds"
        }
        else {
            $stderr = $process.StandardError.ReadToEnd()
            if ($process.ExitCode -eq 0) {
                if ($attempt -gt 1) {
                    Write-Utf8TextFile -Path "$OutputPath.retry.txt" -Value "adb binary capture succeeded on attempt $attempt."
                }
                return
            }

            $attemptStderrPath = "$OutputPath.stderr.attempt-$attempt.txt"
            Write-Utf8TextFile -Path $attemptStderrPath -Value $stderr
            $lastError = "adb binary capture failed with exit code $($process.ExitCode); see $attemptStderrPath"
        }

        if ($attempt -le $RetryCount) {
            Start-Sleep -Milliseconds ([Math]::Min(2000, 500 * $attempt))
        }
    }

    Write-Utf8TextFile -Path "$OutputPath.stderr.txt" -Value $lastError
    throw $lastError
}

function Add-AmExtra {
    param(
        [System.Collections.Generic.List[string]]$LaunchArgs,
        [string]$Key,
        [string]$Value
    )

    if ($Value -match '^(?i:true|false)$') {
        $LaunchArgs.Add("--ez")
        $LaunchArgs.Add($Key)
        $LaunchArgs.Add($Value.ToLowerInvariant())
    }
    elseif ($Value -match '^-?\d+$') {
        $LaunchArgs.Add("--ei")
        $LaunchArgs.Add($Key)
        $LaunchArgs.Add($Value)
    }
    elseif ($Value -match '^-?(?:\d+\.\d*|\d*\.\d+)(?:[eE][+-]?\d+)?$') {
        $LaunchArgs.Add("--ef")
        $LaunchArgs.Add($Key)
        $LaunchArgs.Add($Value)
    }
    else {
        $LaunchArgs.Add("--es")
        $LaunchArgs.Add($Key)
        $LaunchArgs.Add((Format-RemoteShellArg -Value $Value))
    }
}

function Format-RemoteShellArg {
    param([string]$Value)

    if ($Value -notmatch '[\s;&|<>$(){}*?!"]' -and -not $Value.Contains("'")) {
        return $Value
    }

    return "'" + ($Value -replace "'", "'\\''") + "'"
}

function Expand-OverrideItems {
    param([string[]]$Items)
    $expanded = @()
    foreach ($item in $Items) {
        foreach ($part in ($item -split ",")) {
            $trimmed = $part.Trim()
            if ($trimmed.Length -ge 2) {
                $first = $trimmed.Substring(0, 1)
                $last = $trimmed.Substring($trimmed.Length - 1, 1)
                if (($first -eq "'" -and $last -eq "'") -or ($first -eq '"' -and $last -eq '"')) {
                    $trimmed = $trimmed.Substring(1, $trimmed.Length - 2)
                }
            }
            if ($trimmed) {
                $expanded += $trimmed
            }
        }
    }
    return $expanded
}

function Convert-Overrides {
    param([string[]]$Items)
    $values = @{}
    foreach ($item in (Expand-OverrideItems -Items $Items)) {
        $parts = $item.Split("=", 2)
        if ($parts.Count -ne 2 -or -not $parts[0]) {
            throw "Override must be key=value: $item"
        }
        $values[$parts[0]] = $parts[1]
    }
    return $values
}

function Capture-PowerSnapshot {
    param(
        [string]$Dir,
        [string]$Prefix
    )

    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "battery") -OutputPath (Join-Path $Dir "$Prefix-battery.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "power") -OutputPath (Join-Path $Dir "$Prefix-power.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "vrpowermanager") -OutputPath (Join-Path $Dir "$Prefix-vrpowermanager.txt")
}

function Read-TextCaptureValue {
    param(
        [string]$Path,
        [string]$Pattern
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }

    $text = Get-Content -Raw -LiteralPath $Path
    $options = [System.Text.RegularExpressions.RegexOptions]::Multiline
    $match = [regex]::Match($text, $Pattern, $options)
    if ($match.Success) {
        return $match.Groups[1].Value.Trim()
    }
    return $null
}

function Test-TextCapturePattern {
    param(
        [string]$Path,
        [string]$Pattern
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return $false
    }

    $text = Get-Content -Raw -LiteralPath $Path
    $options = [System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor [System.Text.RegularExpressions.RegexOptions]::Multiline
    return [regex]::IsMatch($text, $Pattern, $options)
}

function Get-PowerSnapshotState {
    param(
        [string]$Dir,
        [string]$Prefix
    )

    $vrPath = Join-Path $Dir "$Prefix-vrpowermanager.txt"
    $powerPath = Join-Path $Dir "$Prefix-power.txt"

    return [ordered]@{
        prefix = $Prefix
        virtualProximityState = Read-TextCaptureValue -Path $vrPath -Pattern '^Virtual proximity state:\s*(.+)$'
        headsetState = Read-TextCaptureValue -Path $vrPath -Pattern '^State:\s*(.+)$'
        autoSleepTimeMs = Read-TextCaptureValue -Path $vrPath -Pattern '^AutoSleepTime:\s*([-\d]+)\s*ms'
        wakefulness = Read-TextCaptureValue -Path $powerPath -Pattern '^\s*mWakefulness=(.+)$'
        sleepTimeoutMs = Read-TextCaptureValue -Path $powerPath -Pattern '^Sleep timeout:\s*([-\d]+)\s*ms'
        hasStandbyHistory = Test-TextCapturePattern -Path $vrPath -Pattern 'transition from .* to STANDBY|Calling goToSleep'
    }
}

function New-PowerStateSummary {
    param(
        [string]$Dir,
        [string]$BaselinePrefix,
        [string]$FinalPrefix
    )

    $baseline = Get-PowerSnapshotState -Dir $Dir -Prefix $BaselinePrefix
    $final = Get-PowerSnapshotState -Dir $Dir -Prefix $FinalPrefix
    $issues = [System.Collections.Generic.List[string]]::new()

    if ($baseline.virtualProximityState -and $final.virtualProximityState -and $baseline.virtualProximityState -ne $final.virtualProximityState) {
        $issues.Add("virtual proximity changed from $($baseline.virtualProximityState) to $($final.virtualProximityState)")
    }
    if ($final.wakefulness -and $final.wakefulness -ne "Awake") {
        $issues.Add("final wakefulness is $($final.wakefulness)")
    }
    if ($final.headsetState -and $final.headsetState -match 'STANDBY|UNMOUNTED|WAITING') {
        $issues.Add("final headset state is $($final.headsetState)")
    }

    $summary = [ordered]@{
        schemaVersion = "rusty.xr.quest-camera-profile-power-state.v1"
        status = if ($issues.Count -gt 0) { "warning" } else { "ok" }
        baseline = $baseline
        final = $final
        issues = @($issues)
    }

    Write-Utf8TextFile -Path (Join-Path $Dir "power-state-summary.json") -Value ($summary | ConvertTo-Json -Depth 6)
    return $summary
}

function Invoke-ProximityHold {
    param(
        [string]$Dir,
        [int]$DurationMs
    )

    if ($DurationMs -le 0) {
        return
    }

    $arguments = @("-y", $HzdbNpxPackage, "device", "proximity")
    if ($Serial) {
        $arguments += @("--device", $Serial)
    }
    $arguments += @("--disable", "--duration-ms", $DurationMs.ToString())

    try {
        Invoke-ProcessCapture `
            -FileName $Npx `
            -ArgumentList $arguments `
            -OutputPath (Join-Path $Dir "hzdb-proximity-hold.txt") `
            -TimeoutSeconds 120
    }
    catch {
        $proximityHoldLines = @(
            "Timed hzdb proximity hold failed."
            $_.Exception.Message
        )
        Write-Utf8TextFile -Path (Join-Path $Dir "hzdb-proximity-hold.txt") -Value $proximityHoldLines
    }
}

function Resolve-ActivityComponent {
    param(
        [string]$PackageName,
        [string]$ActivityName
    )
    if (-not $ActivityName) {
        return $PackageName
    }
    if ($ActivityName.Contains("/")) {
        return $ActivityName
    }
    if ($ActivityName.StartsWith(".")) {
        return "$PackageName/$PackageName$ActivityName"
    }
    return "$PackageName/$ActivityName"
}

function Invoke-RunValidation {
    param(
        [string]$Dir,
        [string]$Label
    )

    if (-not (Test-Path $Validator)) {
        return
    }

    $imagePath = Join-Path $Dir "$Label-hzdb-screencap.png"
    if (-not (Test-Path $imagePath)) {
        $imagePath = Join-Path $Dir "$Label-screencap.png"
    }
    $logcatPath = Get-ProfileRunLogcatPath -Dir $Dir -Label $Label
    $validationPath = Join-Path $Dir "$Label-validation.json"
    $sequenceDir = Join-Path $Dir "$Label-freshness-frames"

    try {
        $validatorArgs = @(
            $Validator,
            "--image", $imagePath,
            "--logcat", $logcatPath,
            "--label", $Label,
            "--out", $validationPath
        )
        if (Test-Path -LiteralPath $sequenceDir) {
            $validatorArgs += @("--sequence-dir", $sequenceDir)
        }
        Write-Utf8TextFile -Path (Join-Path $Dir "$Label-validation-stdout.txt") -Value ((& python @validatorArgs) -join [Environment]::NewLine)
    }
    catch {
        $validationErrorLines = @(
            "validation failed"
            $_.Exception.Message
        )
        Write-Utf8TextFile -Path (Join-Path $Dir "$Label-validation-error.txt") -Value $validationErrorLines
    }
}

function Invoke-ProjectionRuntimeReadbackValidation {
    param(
        [string]$Dir,
        [string]$Label,
        [string]$ManifestPath
    )

    $outPath = Join-Path $Dir "projection-runtime-readback.json"
    $stdoutPath = Join-Path $Dir "projection-runtime-readback-stdout.txt"
    $errorPath = Join-Path $Dir "projection-runtime-readback-error.txt"

    if ($ProjectionRuntimeReadback -eq "skip") {
        $skipped = [ordered]@{
            schemaVersion = "rusty.xr.projection-runtime-readback.v1"
            status = "skipped"
            mode = $ProjectionRuntimeReadback
            report = $outPath
            error = ""
        }
        Write-Utf8TextFile -Path $outPath -Value ($skipped | ConvertTo-Json -Depth 5)
        return $skipped
    }

    if (-not (Test-Path -LiteralPath $ProjectionRuntimeReadbackValidator)) {
        $missing = [ordered]@{
            schemaVersion = "rusty.xr.projection-runtime-readback.v1"
            status = "failed"
            mode = $ProjectionRuntimeReadback
            report = $outPath
            error = "projection runtime readback validator not found: $ProjectionRuntimeReadbackValidator"
        }
        Write-Utf8TextFile -Path $outPath -Value ($missing | ConvertTo-Json -Depth 5)
        if ($ProjectionRuntimeReadback -eq "required") {
            throw $missing.error
        }
        return $missing
    }

    $logcatPath = Get-ProfileRunLogcatPath -Dir $Dir -Label $Label
    $validatorArgs = @(
        $ProjectionRuntimeReadbackValidator,
        "--run-manifest", $ManifestPath,
        "--logcat", $logcatPath,
        "--out", $outPath,
        "--expected-source", "command-line"
    )
    $expectedBackend = Get-ProjectionRuntimeExpectedBackend
    if ($expectedBackend -ne "any") {
        $validatorArgs += @("--expected-backend", $expectedBackend)
    }
    if ($ProjectionRuntimeReadback -eq "warn") {
        $validatorArgs += "--allow-missing-manifest"
    }

    $output = @()
    try {
        $previousErrorActionPreference = $ErrorActionPreference
        try {
            $ErrorActionPreference = "Continue"
            $output = @(& python @validatorArgs 2>&1 | ForEach-Object { [string]$_ })
            $exitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
        Write-Utf8TextFile -Path $stdoutPath -Value $output
        if ($exitCode -ne 0) {
            $message = "projection runtime readback validation failed with exit code $exitCode; see $outPath"
            Write-Utf8TextFile -Path $errorPath -Value $message
            if ($ProjectionRuntimeReadback -eq "required") {
                throw $message
            }
        }
    }
    catch {
        Write-Utf8TextFile -Path $errorPath -Value @("projection runtime readback validation failed", $_.Exception.Message)
        if ($ProjectionRuntimeReadback -eq "required") {
            throw
        }
    }

    if (Test-Path -LiteralPath $outPath) {
        try {
            return Get-Content -Raw -LiteralPath $outPath | ConvertFrom-Json
        }
        catch {
            return [ordered]@{
                schemaVersion = "rusty.xr.projection-runtime-readback.v1"
                status = "failed"
                mode = $ProjectionRuntimeReadback
                report = $outPath
                error = "projection runtime readback report was not readable: $($_.Exception.Message)"
            }
        }
    }

    return [ordered]@{
        schemaVersion = "rusty.xr.projection-runtime-readback.v1"
        status = "failed"
        mode = $ProjectionRuntimeReadback
        report = $outPath
        error = "projection runtime readback report was not written"
    }
}

function Capture-Artifacts {
    param(
        [string]$Dir,
        [string]$Label,
        [string]$Package,
        [switch]$SkipLogcatCapture,
        [switch]$SkipRunValidation
    )

    Invoke-AdbBinaryCapture -Arguments @("exec-out", "screencap", "-p") -OutputPath (Join-Path $Dir "$Label-screencap.png")
    if ($FreshnessFrames -gt 1) {
        $freshnessDir = Join-Path $Dir "$Label-freshness-frames"
        New-Item -ItemType Directory -Force -Path $freshnessDir | Out-Null
        $frames = @()
        for ($index = 0; $index -lt $FreshnessFrames; $index++) {
            $framePath = Join-Path $freshnessDir ("frame-{0:D2}.png" -f $index)
            Invoke-AdbBinaryCapture -Arguments @("exec-out", "screencap", "-p") -OutputPath $framePath
            $frames += [pscustomobject][ordered]@{
                index = $index
                path = $framePath
                sha256 = Get-FileSha256Hex -Path $framePath
                bytes = Get-FileByteLength -Path $framePath
            }
            if ($index -lt ($FreshnessFrames - 1) -and $FreshnessIntervalMs -gt 0) {
                Start-Sleep -Milliseconds $FreshnessIntervalMs
            }
        }
        $hashGroups = @($frames | Group-Object -Property sha256)
        $duplicateGroups = @(
            $hashGroups |
                Where-Object { $_.Count -gt 1 } |
                ForEach-Object {
                    [ordered]@{
                        sha256 = $_.Name
                        count = $_.Count
                        indices = @($_.Group | ForEach-Object { $_.index })
                    }
                }
        )
        $freshnessSummary = [ordered]@{
            schemaVersion = "rusty.xr.quest-camera-screenshot-freshness.v1"
            frameCount = $FreshnessFrames
            intervalMs = $FreshnessIntervalMs
            uniqueSha256Count = $hashGroups.Count
            duplicateSha256Groups = $duplicateGroups
            byteIdenticalFreezeSuspected = $duplicateGroups.Count -gt 0
            frames = $frames
        }
        Write-Utf8TextFile -Path (Join-Path $Dir "$Label-freshness-summary.json") -Value ($freshnessSummary | ConvertTo-Json -Depth 6)
    }
    if ($CaptureHzdbScreencap) {
        $arguments = @("-y", $HzdbNpxPackage, "capture", "screenshot")
        if ($Serial) {
            $arguments += @("--device", $Serial)
        }
        $arguments += @("--method", "screencap", "--output", (Join-Path $Dir "$Label-hzdb-screencap.png"))
        try {
            Invoke-ProcessCapture -FileName $Npx -ArgumentList $arguments -OutputPath (Join-Path $Dir "$Label-hzdb-screencap-capture.txt") -TimeoutSeconds 120
        }
        catch {
            Write-Utf8TextFile -Path (Join-Path $Dir "$Label-hzdb-screencap-capture.txt") -Value @("hzdb screencap failed."; $_.Exception.Message)
        }
    }
    if ($CaptureMetacam) {
        $arguments = @("-y", $HzdbNpxPackage, "capture", "screenshot")
        if ($Serial) {
            $arguments += @("--device", $Serial)
        }
        $arguments += @("--method", "metacam", "--output", (Join-Path $Dir "$Label-metacam.png"))
        try {
            Invoke-ProcessCapture -FileName $Npx -ArgumentList $arguments -OutputPath (Join-Path $Dir "$Label-metacam-capture.txt") -TimeoutSeconds 120
        }
        catch {
            Write-Utf8TextFile -Path (Join-Path $Dir "$Label-metacam-capture.txt") -Value @("hzdb metacam capture failed."; $_.Exception.Message)
        }
    }

    if (-not $SkipLogcatCapture) {
        Save-AdbTextCapture -Arguments @("logcat", "-d", "-t", $LogcatLines.ToString()) -OutputPath (Get-ProfileRunLogcatPath -Dir $Dir -Label $Label)
    }
    Save-AdbTextCapture -Arguments @("shell", "pidof", $Package) -OutputPath (Join-Path $Dir "$Label-pid.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "activity", "activities") -OutputPath (Join-Path $Dir "$Label-activity.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "window") -OutputPath (Join-Path $Dir "$Label-window.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "battery") -OutputPath (Join-Path $Dir "$Label-battery.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "power") -OutputPath (Join-Path $Dir "$Label-power.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "vrpowermanager") -OutputPath (Join-Path $Dir "$Label-vrpowermanager.txt")
    Save-OptionalRunAsFileCapture -Package $Package -RemotePath "files/camera-source-diagnostics.json" -OutputPath (Join-Path $Dir "$Label-camera-source-diagnostics.json")
    Save-OptionalRunAsFileCapture -Package $Package -RemotePath "files/controller-tuning-state.json" -OutputPath (Join-Path $Dir "$Label-controller-tuning-state.json")
    if (-not $SkipRunValidation) {
        Invoke-RunValidation -Dir $Dir -Label $Label
    }
}

$catalogPath = Resolve-InputPath $Catalog
$runRootPath = Resolve-InputPath $RunRoot
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$dir = Join-Path $runRootPath "$timestamp-$RuntimeProfile"
New-Item -ItemType Directory -Force -Path $dir | Out-Null

$catalogObject = Get-Content -Raw -LiteralPath $catalogPath | ConvertFrom-Json
$app = $catalogObject.apps | Where-Object { $_.id -eq $AppId } | Select-Object -First 1
if (-not $app) {
    throw "App '$AppId' not found in $catalogPath"
}
$runtimeProfileEntry = $catalogObject.runtimeProfiles | Where-Object { $_.id -eq $RuntimeProfile } | Select-Object -First 1
if (-not $runtimeProfileEntry) {
    throw "Runtime profile '$RuntimeProfile' not found in $catalogPath"
}
$device = $catalogObject.deviceProfiles | Where-Object { $_.id -eq $DeviceProfile } | Select-Object -First 1
if ($DeviceProfile -and -not $device) {
    throw "Device profile '$DeviceProfile' not found in $catalogPath"
}

$packageName = [string]$app.packageName
$component = Resolve-ActivityComponent -PackageName $packageName -ActivityName ([string]$app.activityName)
if ($LaunchActivity) {
    $component = Resolve-ActivityComponent -PackageName $packageName -ActivityName $LaunchActivity
}
$values = @{}
foreach ($property in $runtimeProfileEntry.values.PSObject.Properties) {
    $values[$property.Name] = [string]$property.Value
}
foreach ($entry in (Convert-Overrides -Items $Override).GetEnumerator()) {
    $values[$entry.Key] = [string]$entry.Value
}
if ($CameraPipelinePreset) {
    $values["rustyxr.cameraPipelinePreset"] = $CameraPipelinePreset
}
if ($CameraProjectionMode) {
    $values["rustyxr.cameraProjectionMode"] = $CameraProjectionMode
}

Write-Utf8TextFile -Path (Join-Path $dir "adb-devices.txt") -Value ((Invoke-Adb -Arguments @("devices")) -join [Environment]::NewLine)
$projectionPropertyHygieneSummary = Invoke-RustyXrProjectionPropertyHygiene `
    -Adb $Adb `
    -Serial $Serial `
    -Mode $ProjectionPropertyHygiene `
    -OutputPath (Join-Path $dir "projection-property-hygiene.json")

if ($Install) {
    $apkPath = $Apk
    if (-not $apkPath) {
        $catalogDir = Split-Path -Parent $catalogPath
        $apkPath = [System.IO.Path]::GetFullPath((Join-Path $catalogDir ([string]$app.apkFile)))
    }
    if (-not (Test-Path $apkPath)) {
        throw "APK not found: $apkPath"
    }
    Save-AdbTextCapture -Arguments @("install", "-r", $apkPath) -OutputPath (Join-Path $dir "install.txt")
}

Invoke-Adb -Arguments @("shell", "pm", "grant", $packageName, "android.permission.CAMERA") | Out-Null
Invoke-Adb -Arguments @("shell", "pm", "grant", $packageName, "horizonos.permission.HEADSET_CAMERA") | Out-Null
if ($values.ContainsKey("rustyxr.mediaProjection") -and (Test-TruthyLaunchValue -Value $values["rustyxr.mediaProjection"])) {
    Grant-MediaProjectionAppOp -PackageName $packageName -Dir $dir
}

if ($device) {
    foreach ($property in $device.properties) {
        Save-AdbTextCapture -Arguments @("shell", "setprop", ([string]$property.key), ([string]$property.value)) -OutputPath (Join-Path $dir "setprop-$($property.key).txt")
    }
}

Capture-PowerSnapshot -Dir $dir -Prefix "preflight"
Invoke-Adb -Arguments @("shell", "am", "force-stop", $packageName) | Out-Null
Invoke-Adb -Arguments @("logcat", "-c") | Out-Null
if ($proximityHoldRequested) {
    Invoke-ProximityHold -Dir $dir -DurationMs $ProximityHoldDurationMs
}
Capture-PowerSnapshot -Dir $dir -Prefix "post-proximity-hold"

$label = $RuntimeProfile
$logcatWindowCapture = $null

$launchArgs = [System.Collections.Generic.List[string]]::new()
foreach ($item in @("shell", "am", "start", "-S", "-a", "android.intent.action.MAIN", "-c", $LaunchCategory, "-n", $component)) {
    $launchArgs.Add($item)
}
foreach ($key in ($values.Keys | Sort-Object)) {
    Add-AmExtra -LaunchArgs $launchArgs -Key $key -Value $values[$key]
}

Write-Utf8TextFile -Path (Join-Path $dir "launch-command.txt") -Value ($launchArgs -join " ")
$logcatWindowCapture = Start-AdbLogcatWindowCapture -Dir $dir -Label $label
try {
    Save-AdbTextCapture -Arguments $launchArgs.ToArray() -OutputPath (Join-Path $dir "launch.txt")

    Start-Sleep -Seconds $WarmupSeconds
    Capture-Artifacts -Dir $dir -Label $label -Package $packageName -SkipLogcatCapture -SkipRunValidation
}
finally {
    $logcatWindowCapture = Stop-AdbLogcatWindowCapture -Capture $logcatWindowCapture
    Write-Utf8TextFile -Path (Join-Path $dir "logcat-window-summary.json") -Value ($logcatWindowCapture | ConvertTo-Json -Depth 5)
}
Invoke-RunValidation -Dir $dir -Label $label
$powerStateSummary = New-PowerStateSummary -Dir $dir -BaselinePrefix "post-proximity-hold" -FinalPrefix $label
if ($powerStateSummary.status -ne "ok") {
    Write-Warning "Power/proximity state drift detected; see $dir\power-state-summary.json."
    if ($FailOnPowerStateDrift) {
        throw "Power/proximity state drift detected."
    }
}

$manifest = [ordered]@{
    schemaVersion = "rusty.xr.quest-camera-profile-run.v1"
    capturedAt = (Get-Date).ToString("o")
    serial = $Serial
    catalog = $catalogPath
    appId = $AppId
    packageName = $packageName
    component = $component
    launchCategory = $LaunchCategory
    launchActivityOverride = $LaunchActivity
    deviceProfile = $DeviceProfile
    runtimeProfile = $RuntimeProfile
    cameraPipelinePreset = $CameraPipelinePreset
    cameraProjectionMode = $CameraProjectionMode
    warmupSeconds = $WarmupSeconds
    proximityHoldRequested = $proximityHoldRequested
    proximityHoldDurationMs = if ($proximityHoldRequested) { $ProximityHoldDurationMs } else { 0 }
    captureHzdbScreencap = [bool]$CaptureHzdbScreencap
    captureMetacam = [bool]$CaptureMetacam
    freshnessFrames = $FreshnessFrames
    freshnessIntervalMs = $FreshnessIntervalMs
    failOnPowerStateDrift = [bool]$FailOnPowerStateDrift
    projectionPropertyHygiene = $projectionPropertyHygieneSummary
    projectionRuntimeReadbackMode = $ProjectionRuntimeReadback
    logcatCapture = $logcatWindowCapture
    logcatLines = $LogcatLines
    overrides = $Override
    values = $values
    artifactDir = $dir
    powerState = $powerStateSummary
    validations = @(Get-ChildItem -LiteralPath $dir -Filter "*-validation.json" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            Get-Content -Raw -LiteralPath $_.FullName | ConvertFrom-Json
        }
        catch {
            [ordered]@{
                status = "unreadable"
                path = $_.FullName
            }
        }
    })
}
$manifestPath = Join-Path $dir "run-manifest.json"
Write-Utf8TextFile -Path $manifestPath -Value ($manifest | ConvertTo-Json -Depth 6)
$manifest["projectionRuntimeReadback"] = Invoke-ProjectionRuntimeReadbackValidation -Dir $dir -Label $label -ManifestPath $manifestPath
Write-Utf8TextFile -Path $manifestPath -Value ($manifest | ConvertTo-Json -Depth 8)
Write-Output $dir
