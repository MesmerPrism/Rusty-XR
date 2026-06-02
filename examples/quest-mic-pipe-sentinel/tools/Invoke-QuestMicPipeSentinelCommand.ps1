<#
.SYNOPSIS
    Sends development commands to the Quest mic-pipe sentinel panel.

.DESCRIPTION
    This wrapper drives the same visible Activity routines as the panel
    buttons through launch extras. It avoids UI tree dumps and coordinate taps
    while preserving Android's visible-panel and microphone-permission policy.
#>
[CmdletBinding()]
param(
    [ValidateSet('Show', 'RequestPermissions', 'GrantTermuxPermission', 'RequestTermuxPermission', 'StartTermuxReceiver', 'StopTermuxReceiver', 'Start', 'Stop', 'Events')]
    [string]$Command = 'Show',

    [string]$Serial = '',

    [Alias('Host')]
    [string]$ReceiverHost = '127.0.0.1',

    [ValidateRange(1, 65535)]
    [int]$Port = 34567,

    [string]$RunId = '',

    [string]$ReceiverScript = '.\examples\quest-mic-pipe-sentinel\tools\mic_recv_wav.py',

    [string]$ReceiverScriptDevicePath = '/sdcard/Download/rustyxr_mic_recv_wav.py',

    [string]$WavDevicePath = '/sdcard/Download/rustyxr_mic_capture.wav',

    [ValidateRange(1, 3600)]
    [int]$DurationSeconds = 180,

    [switch]$SkipReceiverScriptPush,

    [string]$Adb = '',

    [string]$OutFile = '',

    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($Adb)) {
    $Adb = $env:RUSTY_XR_ADB
}
if ([string]::IsNullOrWhiteSpace($Adb)) {
    $Adb = 'adb'
}

$packageName = 'com.example.rustyxr.micpipe'
$activityName = "$packageName/.MicPanelActivity"

function New-AdbPrefix {
    if ([string]::IsNullOrWhiteSpace($Serial)) {
        return @()
    }
    return @('-s', $Serial)
}

function Invoke-AdbChecked {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    Write-Host "> $Adb $($Arguments -join ' ')"
    if ($DryRun) {
        return
    }
    & $Adb @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "adb command failed with exit code $LASTEXITCODE"
    }
}

if ($Command -eq 'Events') {
    $eventReadArgs = @(New-AdbPrefix) + @(
        'exec-out',
        'run-as',
        $packageName,
        'cat',
        'files/micpipe-events.jsonl'
    )
    Write-Host "> $Adb $($eventReadArgs -join ' ')"
    if ($DryRun) {
        exit 0
    }
    $events = & $Adb @eventReadArgs
    if ($LASTEXITCODE -ne 0) {
        throw "adb command failed with exit code $LASTEXITCODE"
    }
    if ([string]::IsNullOrWhiteSpace($OutFile)) {
        $events
    } else {
        $outPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutFile)
        $events | Set-Content -LiteralPath $outPath -Encoding UTF8
        Write-Host "Wrote events: $outPath"
    }
    exit 0
}

if ($Command -eq 'GrantTermuxPermission') {
    Invoke-AdbChecked -Arguments (@(New-AdbPrefix) + @(
        'shell',
        'pm',
        'grant',
        $packageName,
        'com.termux.permission.RUN_COMMAND'
    ))
    exit 0
}

if ($Command -eq 'StartTermuxReceiver' -and -not $SkipReceiverScriptPush) {
    $receiverScriptPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($ReceiverScript)
    if (-not (Test-Path -LiteralPath $receiverScriptPath)) {
        throw "Receiver script not found: $receiverScriptPath"
    }
    Invoke-AdbChecked -Arguments (@(New-AdbPrefix) + @(
        'push',
        $receiverScriptPath,
        $ReceiverScriptDevicePath
    ))
}

if ([string]::IsNullOrWhiteSpace($RunId) -and ($Command -eq 'Start' -or $Command -eq 'StartTermuxReceiver')) {
    $RunId = 'micpipe-cli-' + (Get-Date -Format 'yyyyMMdd-HHmmss')
}

$androidCommand = switch ($Command) {
    'Show' { 'show' }
    'RequestPermissions' { 'request-permissions' }
    'RequestTermuxPermission' { 'request-termux-permission' }
    'StartTermuxReceiver' { 'start-termux-receiver' }
    'StopTermuxReceiver' { 'stop-termux-receiver' }
    'Start' { 'start' }
    'Stop' { 'stop' }
}

$startArgs = @(New-AdbPrefix) + @(
    'shell',
    'am',
    'start',
    '-W',
    '-n',
    $activityName,
    '--es',
    'rustyxr.micPipe.command',
    $androidCommand
)

if ($Command -eq 'Start' -or $Command -eq 'StartTermuxReceiver' -or -not [string]::IsNullOrWhiteSpace($RunId)) {
    $startArgs += @(
        '--es',
        'rustyxr.micPipe.host',
        $ReceiverHost,
        '--es',
        'rustyxr.micPipe.port',
        $Port.ToString()
    )
    if (-not [string]::IsNullOrWhiteSpace($RunId)) {
        $startArgs += @('--es', 'rustyxr.micPipe.runId', $RunId)
    }
}

if ($Command -eq 'StartTermuxReceiver') {
    $startArgs += @(
        '--es',
        'rustyxr.micPipe.termuxScript',
        $ReceiverScriptDevicePath,
        '--es',
        'rustyxr.micPipe.termuxWav',
        $WavDevicePath,
        '--ei',
        'rustyxr.micPipe.termuxDurationSeconds',
        $DurationSeconds.ToString()
    )
}

Invoke-AdbChecked -Arguments $startArgs
