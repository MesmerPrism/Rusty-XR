<#
.SYNOPSIS
    Sends a runtime-profile intent to the already-running Quest composite APK.
#>
[CmdletBinding()]
param(
    [string]$Serial = '',
    [string]$Catalog = '..\catalog\rusty-xr-quest-composite-layer.catalog.json',
    [string]$AppId = 'rusty-xr-quest-composite-layer',
    [string]$RuntimeProfile = 'passthrough-underlay-hotload-neutral',
    [string[]]$Override = @(),
    [string]$LaunchCategory = 'com.oculus.intent.category.VR',
    [string]$LaunchActivity = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-InputPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return (Resolve-Path $Path).Path
    }
    return (Resolve-Path (Join-Path $PSScriptRoot $Path)).Path
}

function Resolve-ActivityComponent {
    param(
        [string]$PackageName,
        [string]$ActivityName
    )
    if ($ActivityName.StartsWith('.')) {
        return "$PackageName/$PackageName$ActivityName"
    }
    if ($ActivityName.Contains('/')) {
        return $ActivityName
    }
    return "$PackageName/$ActivityName"
}

function Format-RemoteShellArg {
    param([string]$Value)
    if ($Value -notmatch '[\s;&|<>$(){}*?!"]' -and -not $Value.Contains("'")) {
        return $Value
    }
    return "'" + ($Value -replace "'", "'\''") + "'"
}

function Add-AmExtra {
    param(
        [System.Collections.Generic.List[string]]$LaunchArgs,
        [string]$Key,
        [string]$Value
    )

    if ($Value -match '^(?i:true|false)$') {
        $LaunchArgs.Add('--ez')
        $LaunchArgs.Add($Key)
        $LaunchArgs.Add($Value.ToLowerInvariant())
    }
    elseif ($Value -match '^-?\d+$') {
        $LaunchArgs.Add('--ei')
        $LaunchArgs.Add($Key)
        $LaunchArgs.Add($Value)
    }
    elseif ($Value -match '^-?(?:\d+\.\d*|\d*\.\d+)(?:[eE][+-]?\d+)?$') {
        $LaunchArgs.Add('--ef')
        $LaunchArgs.Add($Key)
        $LaunchArgs.Add($Value)
    }
    else {
        $LaunchArgs.Add('--es')
        $LaunchArgs.Add($Key)
        $LaunchArgs.Add((Format-RemoteShellArg -Value $Value))
    }
}

function Convert-Overrides {
    param([string[]]$Items)
    $values = @{}
    foreach ($item in $Items) {
        foreach ($part in ($item -split ',')) {
            $trimmed = $part.Trim()
            if (-not $trimmed) {
                continue
            }
            $separator = $trimmed.IndexOf('=')
            if ($separator -lt 1) {
                throw "Override must be key=value: $trimmed"
            }
            $key = $trimmed.Substring(0, $separator).Trim()
            $value = $trimmed.Substring($separator + 1).Trim()
            $values[$key] = $value
        }
    }
    return $values
}

function Get-ProfileNumber {
    param(
        [hashtable]$Values,
        [string]$Key
    )
    if (-not $Values.ContainsKey($Key)) {
        return 0.0
    }
    $number = 0.0
    if ([double]::TryParse(
            [string]$Values[$Key],
            [System.Globalization.NumberStyles]::Float,
            [System.Globalization.CultureInfo]::InvariantCulture,
            [ref]$number)) {
        return $number
    }
    return 0.0
}

function Invoke-Adb {
    param([string[]]$Arguments)
    $adbArgs = @()
    if ($Serial) {
        $adbArgs += @('-s', $Serial)
    }
    $adbArgs += $Arguments
    & adb @adbArgs
    if ($LASTEXITCODE -ne 0) {
        throw "adb failed with exit code $LASTEXITCODE"
    }
}

$catalogPath = Resolve-InputPath -Path $Catalog
$catalogObject = Get-Content -Raw -LiteralPath $catalogPath | ConvertFrom-Json
$app = $catalogObject.apps | Where-Object { $_.id -eq $AppId } | Select-Object -First 1
if (-not $app) {
    throw "App '$AppId' not found in $catalogPath"
}
$profile = $catalogObject.runtimeProfiles | Where-Object { $_.id -eq $RuntimeProfile } | Select-Object -First 1
if (-not $profile) {
    throw "Runtime profile '$RuntimeProfile' not found in $catalogPath"
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

$fullFieldFlickerHz = Get-ProfileNumber -Values $values -Key 'rustyxr.fullFieldFlickerHz'
$passthroughLutFlickerHz = Get-ProfileNumber -Values $values -Key 'rustyxr.passthroughLutFlickerHz'
if ($fullFieldFlickerHz -gt 0.0 -or $passthroughLutFlickerHz -gt 0.0) {
    Write-Warning 'This runtime profile intentionally uses strobing light. It can trigger seizures or other adverse reactions in people with photosensitive epilepsy or light-sensitive conditions. Use only with explicit informed opt-in.'
}

$launchArgs = [System.Collections.Generic.List[string]]::new()
foreach ($item in @('shell', 'am', 'start', '-a', 'android.intent.action.MAIN', '-c', $LaunchCategory, '-n', $component)) {
    $launchArgs.Add($item)
}
foreach ($key in ($values.Keys | Sort-Object)) {
    Add-AmExtra -LaunchArgs $launchArgs -Key $key -Value $values[$key]
}

Write-Host "> adb $($launchArgs -join ' ')"
Invoke-Adb -Arguments $launchArgs.ToArray()
