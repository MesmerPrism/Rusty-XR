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
    [switch]$SkipProximityHold,
    [int]$ProximityHoldDurationMs = 600000,
    [switch]$CaptureHzdbScreencap,
    [switch]$CaptureMetacam,
    [switch]$FailOnPowerStateDrift,
    [int]$LogcatLines = 12000,
    [string]$Validator = ""
)

$ErrorActionPreference = "Stop"

if (-not $Validator) {
    $Validator = Join-Path $PSScriptRoot "Validate-QuestCameraRun.py"
}

function Resolve-InputPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))
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
    Invoke-Adb -Arguments $Arguments | Out-File -FilePath $OutputPath -Encoding UTF8
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
    $startInfo.FileName = $FileName
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
    @(
        "exitCode=$($process.ExitCode)"
        ""
        "[stdout]"
        $stdout
        ""
        "[stderr]"
        $stderr
    ) | Set-Content -Path $OutputPath -Encoding UTF8

    if ($process.ExitCode -ne 0) {
        throw "$FileName failed with exit code $($process.ExitCode); see $OutputPath"
    }
}

function Invoke-AdbBinaryCapture {
    param(
        [string[]]$Arguments,
        [string]$OutputPath,
        [int]$TimeoutSeconds = 30
    )

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
    $startInfo.FileName = $Adb
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

    $fileStream = [System.IO.File]::Create($OutputPath)
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
        throw "adb binary capture timed out after $TimeoutSeconds seconds"
    }

    $stderr = $process.StandardError.ReadToEnd()
    if ($process.ExitCode -ne 0) {
        $stderrPath = "$OutputPath.stderr.txt"
        Set-Content -Path $stderrPath -Value $stderr -Encoding UTF8
        throw "adb binary capture failed with exit code $($process.ExitCode); see $stderrPath"
    }
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

    $summary | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $Dir "power-state-summary.json") -Encoding UTF8
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
        @(
            "Timed hzdb proximity hold failed."
            $_.Exception.Message
        ) | Set-Content -Path (Join-Path $Dir "hzdb-proximity-hold.txt") -Encoding UTF8
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
    $logcatPath = Join-Path $Dir "$Label-logcat-tail.txt"
    $validationPath = Join-Path $Dir "$Label-validation.json"

    try {
        python $Validator --image $imagePath --logcat $logcatPath --label $Label --out $validationPath | Out-File -FilePath (Join-Path $Dir "$Label-validation-stdout.txt") -Encoding UTF8
    }
    catch {
        @(
            "validation failed"
            $_.Exception.Message
        ) | Set-Content -Path (Join-Path $Dir "$Label-validation-error.txt") -Encoding UTF8
    }
}

function Capture-Artifacts {
    param(
        [string]$Dir,
        [string]$Label,
        [string]$Package
    )

    Invoke-AdbBinaryCapture -Arguments @("exec-out", "screencap", "-p") -OutputPath (Join-Path $Dir "$Label-screencap.png")
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
            @("hzdb screencap failed."; $_.Exception.Message) | Set-Content -Path (Join-Path $Dir "$Label-hzdb-screencap-capture.txt") -Encoding UTF8
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
            @("hzdb metacam capture failed."; $_.Exception.Message) | Set-Content -Path (Join-Path $Dir "$Label-metacam-capture.txt") -Encoding UTF8
        }
    }

    Save-AdbTextCapture -Arguments @("logcat", "-d", "-t", $LogcatLines.ToString()) -OutputPath (Join-Path $Dir "$Label-logcat-tail.txt")
    Save-AdbTextCapture -Arguments @("shell", "pidof", $Package) -OutputPath (Join-Path $Dir "$Label-pid.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "activity", "activities") -OutputPath (Join-Path $Dir "$Label-activity.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "window") -OutputPath (Join-Path $Dir "$Label-window.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "battery") -OutputPath (Join-Path $Dir "$Label-battery.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "power") -OutputPath (Join-Path $Dir "$Label-power.txt")
    Save-AdbTextCapture -Arguments @("shell", "dumpsys", "vrpowermanager") -OutputPath (Join-Path $Dir "$Label-vrpowermanager.txt")
    Save-AdbTextCapture -Arguments @("shell", "run-as", $Package, "cat", "files/camera-source-diagnostics.json") -OutputPath (Join-Path $Dir "$Label-camera-source-diagnostics.json")
    Invoke-RunValidation -Dir $Dir -Label $Label
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
$profile = $catalogObject.runtimeProfiles | Where-Object { $_.id -eq $RuntimeProfile } | Select-Object -First 1
if (-not $profile) {
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
foreach ($property in $profile.values.PSObject.Properties) {
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

Invoke-Adb -Arguments @("devices") | Out-File -FilePath (Join-Path $dir "adb-devices.txt") -Encoding UTF8

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

if ($device) {
    foreach ($property in $device.properties) {
        Save-AdbTextCapture -Arguments @("shell", "setprop", ([string]$property.key), ([string]$property.value)) -OutputPath (Join-Path $dir "setprop-$($property.key).txt")
    }
}

Capture-PowerSnapshot -Dir $dir -Prefix "preflight"
Invoke-Adb -Arguments @("shell", "am", "force-stop", $packageName) | Out-Null
Invoke-Adb -Arguments @("logcat", "-c") | Out-Null
if (-not $SkipProximityHold) {
    Invoke-ProximityHold -Dir $dir -DurationMs $ProximityHoldDurationMs
}
Capture-PowerSnapshot -Dir $dir -Prefix "post-proximity-hold"

$launchArgs = [System.Collections.Generic.List[string]]::new()
foreach ($item in @("shell", "am", "start", "-S", "-a", "android.intent.action.MAIN", "-c", $LaunchCategory, "-n", $component)) {
    $launchArgs.Add($item)
}
foreach ($key in ($values.Keys | Sort-Object)) {
    Add-AmExtra -LaunchArgs $launchArgs -Key $key -Value $values[$key]
}

($launchArgs -join " ") | Set-Content -Path (Join-Path $dir "launch-command.txt") -Encoding ASCII
Save-AdbTextCapture -Arguments $launchArgs.ToArray() -OutputPath (Join-Path $dir "launch.txt")

Start-Sleep -Seconds $WarmupSeconds
$label = $RuntimeProfile
Capture-Artifacts -Dir $dir -Label $label -Package $packageName
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
    proximityHoldDurationMs = if ($SkipProximityHold) { 0 } else { $ProximityHoldDurationMs }
    captureHzdbScreencap = [bool]$CaptureHzdbScreencap
    captureMetacam = [bool]$CaptureMetacam
    failOnPowerStateDrift = [bool]$FailOnPowerStateDrift
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
$manifest | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $dir "run-manifest.json") -Encoding UTF8
Write-Output $dir
