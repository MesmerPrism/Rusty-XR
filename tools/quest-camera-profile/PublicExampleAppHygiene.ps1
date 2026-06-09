function Get-RustyXrPublicExampleXrPackages {
    param([string[]]$AdditionalPackageNames = @())

    $names = [System.Collections.Generic.List[string]]::new()
    $names.Add("com.example.rustyxr.composite")
    $names.Add("com.example.rustyxr.opengles")
    $names.Add("io.github.mesmerprism.rustyquest.makepad.camera")
    foreach ($packageNameEntry in @($AdditionalPackageNames)) {
        if (-not [string]::IsNullOrWhiteSpace($packageNameEntry)) {
            $names.Add($packageNameEntry)
        }
    }

    return @($names | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
}

function Invoke-RustyXrPublicExampleSiblingForceStop {
    param(
        [string]$Adb = "adb",
        [string]$Serial = "",
        [string]$ActivePackageName = "",
        [string[]]$PackageNames = @(),
        [string]$OutputPath = "",
        [switch]$Skip
    )

    if ($Skip) {
        $summary = [ordered]@{
            skipped = $true
            activePackageName = $ActivePackageName
            packages = @()
            records = @()
        }
        if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
            $summary | ConvertTo-Json -Depth 6 |
                Set-Content -Path $OutputPath -Encoding UTF8
        }
        return [pscustomobject]$summary
    }

    $candidatePackages = if ($PackageNames.Count -gt 0) {
        @($PackageNames)
    }
    else {
        @(Get-RustyXrPublicExampleXrPackages)
    }
    $packagesToStop = @(
        $candidatePackages |
            Where-Object {
                -not [string]::IsNullOrWhiteSpace($_) -and
                    ($_.Trim() -ne $ActivePackageName)
            } |
            Sort-Object -Unique
    )

    $nativePreferenceWasPresent = Test-Path variable:PSNativeCommandUseErrorActionPreference
    $oldNativePreference = if ($nativePreferenceWasPresent) {
        $PSNativeCommandUseErrorActionPreference
    }
    else {
        $null
    }
    if ($nativePreferenceWasPresent) {
        $script:PSNativeCommandUseErrorActionPreference = $false
    }

    try {
        $records = foreach ($packageNameItem in $packagesToStop) {
            $safePackageName = [string]$packageNameItem
            $pidArgs = @()
            $stopArgs = @()
            if (-not [string]::IsNullOrWhiteSpace($Serial)) {
                $pidArgs += @("-s", $Serial)
                $stopArgs += @("-s", $Serial)
            }
            $pidArgs += @("shell", "pidof", $safePackageName)
            $stopArgs += @("shell", "am", "force-stop", $safePackageName)

            $pidBeforeOutput = @(& $Adb @pidArgs 2>&1)
            $pidBeforeExitCode = $LASTEXITCODE
            $forceStopOutput = @(& $Adb @stopArgs 2>&1)
            $forceStopExitCode = $LASTEXITCODE
            $pidAfterOutput = @(& $Adb @pidArgs 2>&1)
            $pidAfterExitCode = $LASTEXITCODE

            [pscustomobject]@{
                packageName = $safePackageName
                pidBefore = (($pidBeforeOutput) -join " ").Trim()
                pidBeforeExitCode = $pidBeforeExitCode
                forceStopOutput = $forceStopOutput
                forceStopExitCode = $forceStopExitCode
                pidAfter = (($pidAfterOutput) -join " ").Trim()
                pidAfterExitCode = $pidAfterExitCode
            }
        }
    }
    finally {
        if ($nativePreferenceWasPresent) {
            $script:PSNativeCommandUseErrorActionPreference = $oldNativePreference
        }
    }

    $summary = [ordered]@{
        skipped = $false
        activePackageName = $ActivePackageName
        packages = $packagesToStop
        records = @($records)
    }
    if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        $summary | ConvertTo-Json -Depth 8 |
            Set-Content -Path $OutputPath -Encoding UTF8
    }
    return [pscustomobject]$summary
}
