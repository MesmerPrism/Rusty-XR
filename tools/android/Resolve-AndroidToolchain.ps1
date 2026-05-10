Set-StrictMode -Version Latest

function Get-RustyXrEnvironmentValue {
    param([string[]]$Names)

    foreach ($name in $Names) {
        foreach ($target in @(
            [System.EnvironmentVariableTarget]::Process,
            [System.EnvironmentVariableTarget]::User,
            [System.EnvironmentVariableTarget]::Machine
        )) {
            $value = [Environment]::GetEnvironmentVariable($name, $target)
            if (-not [string]::IsNullOrWhiteSpace($value)) {
                return $value
            }
        }
    }

    return ''
}

function Resolve-RustyXrDirectory {
    param(
        [string]$Path,
        [string]$Label
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ''
    }

    try {
        $resolved = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
    } catch {
        throw "$Label does not exist: $Path"
    }

    if (-not (Test-Path -LiteralPath $resolved -PathType Container)) {
        throw "$Label is not a directory: $resolved"
    }

    return $resolved
}

function Test-RustyXrJdkRoot {
    param(
        [string]$JdkRoot,
        [switch]$ProbeExecutable
    )

    if ([string]::IsNullOrWhiteSpace($JdkRoot) -or
        -not (Test-Path -LiteralPath $JdkRoot -PathType Container)) {
        return $false
    }

    foreach ($tool in @('java.exe', 'javac.exe', 'jar.exe', 'keytool.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $JdkRoot "bin\$tool") -PathType Leaf)) {
            return $false
        }
    }

    if ($ProbeExecutable) {
        $javac = Join-Path $JdkRoot 'bin\javac.exe'
        try {
            & $javac '-version' *> $null
            if ($LASTEXITCODE -ne 0) {
                return $false
            }
        } catch {
            return $false
        }
    }

    return $true
}

function Assert-RustyXrJdkRoot {
    param([string]$JdkRoot)

    if (-not (Test-RustyXrJdkRoot -JdkRoot $JdkRoot -ProbeExecutable)) {
        throw "JDK root is missing required tools or javac cannot run: $JdkRoot"
    }
}

function Test-RustyXrAndroidSdkRoot {
    param([string]$SdkRoot)

    return -not [string]::IsNullOrWhiteSpace($SdkRoot) -and
        (Test-Path -LiteralPath $SdkRoot -PathType Container) -and
        (Test-Path -LiteralPath (Join-Path $SdkRoot 'build-tools') -PathType Container) -and
        (Test-Path -LiteralPath (Join-Path $SdkRoot 'platforms') -PathType Container)
}

function Assert-RustyXrAndroidSdkRoot {
    param([string]$SdkRoot)

    if (-not (Test-RustyXrAndroidSdkRoot -SdkRoot $SdkRoot)) {
        throw "Android SDK root must contain build-tools and platforms: $SdkRoot"
    }
}

function Test-RustyXrAndroidNdkRoot {
    param([string]$NdkRoot)

    return -not [string]::IsNullOrWhiteSpace($NdkRoot) -and
        (Test-Path -LiteralPath $NdkRoot -PathType Container) -and
        (Test-Path -LiteralPath (Join-Path $NdkRoot 'toolchains\llvm\prebuilt') -PathType Container)
}

function Find-RustyXrDefaultNdkRoot {
    param([string]$SdkRoot)

    if ([string]::IsNullOrWhiteSpace($SdkRoot)) {
        return ''
    }

    $sideBySideRoot = Join-Path $SdkRoot 'ndk'
    if (Test-Path -LiteralPath $sideBySideRoot -PathType Container) {
        $match = Get-ChildItem -LiteralPath $sideBySideRoot -Directory |
            Sort-Object Name -Descending |
            Select-Object -First 1
        if ($null -ne $match -and (Test-RustyXrAndroidNdkRoot -NdkRoot $match.FullName)) {
            return $match.FullName
        }
    }

    $bundleRoot = Join-Path $SdkRoot 'ndk-bundle'
    if (Test-RustyXrAndroidNdkRoot -NdkRoot $bundleRoot) {
        return $bundleRoot
    }

    return ''
}

function Get-RustyXrLatestAndroidDirectory {
    param(
        [string]$Parent,
        [string]$Pattern
    )

    $directory = Get-ChildItem -LiteralPath $Parent -Directory -Filter $Pattern |
        Sort-Object Name -Descending |
        Select-Object -First 1
    if ($null -eq $directory) {
        throw "No directory matching $Pattern under $Parent"
    }

    return $directory.FullName
}

function New-RustyXrAndroidToolchain {
    param(
        [string]$AndroidPlayerRoot,
        [string]$SdkRoot,
        [string]$NdkRoot,
        [string]$JdkRoot,
        [switch]$RequireNdk
    )

    Assert-RustyXrAndroidSdkRoot -SdkRoot $SdkRoot
    Assert-RustyXrJdkRoot -JdkRoot $JdkRoot

    if ($RequireNdk -and -not (Test-RustyXrAndroidNdkRoot -NdkRoot $NdkRoot)) {
        throw "Android NDK root must contain an LLVM toolchain: $NdkRoot"
    }

    [pscustomobject]@{
        AndroidPlayerRoot = $AndroidPlayerRoot
        SdkRoot = $SdkRoot
        NdkRoot = $NdkRoot
        JdkRoot = $JdkRoot
    }
}

function Resolve-RustyXrAndroidToolchain {
    param(
        [string]$AndroidPlayerRoot = '',
        [string]$AndroidSdkRoot = '',
        [string]$AndroidNdkRoot = '',
        [string]$JdkRoot = '',
        [switch]$RequireNdk
    )

    $explicitSdk = Resolve-RustyXrDirectory `
        -Path $(if (-not [string]::IsNullOrWhiteSpace($AndroidSdkRoot)) {
                $AndroidSdkRoot
            } else {
                Get-RustyXrEnvironmentValue -Names @('RUSTY_XR_ANDROID_SDK_ROOT', 'ANDROID_SDK_ROOT', 'ANDROID_HOME')
            }) `
        -Label 'Android SDK root'
    $explicitNdk = Resolve-RustyXrDirectory `
        -Path $(if (-not [string]::IsNullOrWhiteSpace($AndroidNdkRoot)) {
                $AndroidNdkRoot
            } else {
                Get-RustyXrEnvironmentValue -Names @('RUSTY_XR_ANDROID_NDK_ROOT', 'ANDROID_NDK_ROOT', 'ANDROID_NDK_HOME')
            }) `
        -Label 'Android NDK root'
    $explicitJdk = Resolve-RustyXrDirectory `
        -Path $(if (-not [string]::IsNullOrWhiteSpace($JdkRoot)) {
                $JdkRoot
            } else {
                Get-RustyXrEnvironmentValue -Names @('RUSTY_XR_ANDROID_JDK_ROOT', 'JAVA_HOME')
            }) `
        -Label 'JDK root'

    if (-not [string]::IsNullOrWhiteSpace($explicitSdk) -and
        -not [string]::IsNullOrWhiteSpace($explicitJdk)) {
        if ([string]::IsNullOrWhiteSpace($explicitNdk)) {
            $explicitNdk = Find-RustyXrDefaultNdkRoot -SdkRoot $explicitSdk
        }

        return New-RustyXrAndroidToolchain `
            -AndroidPlayerRoot '' `
            -SdkRoot $explicitSdk `
            -NdkRoot $explicitNdk `
            -JdkRoot $explicitJdk `
            -RequireNdk:$RequireNdk
    }

    $playerCandidates = @()
    if (-not [string]::IsNullOrWhiteSpace($AndroidPlayerRoot)) {
        $playerCandidates += $AndroidPlayerRoot
    } else {
        foreach ($envName in @('UNITY_ANDROID_PLAYER_ROOT', 'ANDROID_PLAYER_ROOT')) {
            $value = [Environment]::GetEnvironmentVariable($envName)
            if (-not [string]::IsNullOrWhiteSpace($value)) {
                $playerCandidates += $value
            }
        }

        $unityRoot = Join-Path $env:ProgramFiles 'Unity\Hub\Editor'
        if (Test-Path -LiteralPath $unityRoot -PathType Container) {
            $playerCandidates += Get-ChildItem -LiteralPath $unityRoot -Directory |
                ForEach-Object { Join-Path $_.FullName 'Editor\Data\PlaybackEngines\AndroidPlayer' } |
                Where-Object { Test-Path -LiteralPath $_ -PathType Container } |
                Sort-Object -Descending
        }
    }

    $errors = @()
    foreach ($candidateRoot in $playerCandidates) {
        try {
            $resolvedRoot = Resolve-RustyXrDirectory -Path $candidateRoot -Label 'Android player root'
            $sdkRoot = Join-Path $resolvedRoot 'SDK'
            $ndkRoot = if (-not [string]::IsNullOrWhiteSpace($explicitNdk)) {
                $explicitNdk
            } else {
                Join-Path $resolvedRoot 'NDK'
            }
            $candidateJdk = if (-not [string]::IsNullOrWhiteSpace($explicitJdk)) {
                $explicitJdk
            } else {
                Join-Path $resolvedRoot 'OpenJDK'
            }

            if (-not (Test-RustyXrAndroidSdkRoot -SdkRoot $sdkRoot)) {
                throw "missing SDK/build-tools/platforms"
            }
            if ($RequireNdk -and -not (Test-RustyXrAndroidNdkRoot -NdkRoot $ndkRoot)) {
                throw "missing NDK LLVM toolchain"
            }
            if (-not (Test-RustyXrJdkRoot -JdkRoot $candidateJdk -ProbeExecutable)) {
                throw "missing or broken JDK; set -JdkRoot or RUSTY_XR_ANDROID_JDK_ROOT to a working JDK"
            }

            return New-RustyXrAndroidToolchain `
                -AndroidPlayerRoot $resolvedRoot `
                -SdkRoot $sdkRoot `
                -NdkRoot $ndkRoot `
                -JdkRoot $candidateJdk `
                -RequireNdk:$RequireNdk
        } catch {
            $errors += "$candidateRoot ($($_.Exception.Message))"
        }
    }

    $message = 'Could not find Android tooling. Use -AndroidPlayerRoot for a Unity Android player root, or provide split roots with -AndroidSdkRoot/-AndroidNdkRoot/-JdkRoot. Environment fallbacks are RUSTY_XR_ANDROID_SDK_ROOT, RUSTY_XR_ANDROID_NDK_ROOT, RUSTY_XR_ANDROID_JDK_ROOT, ANDROID_SDK_ROOT, ANDROID_NDK_ROOT, and JAVA_HOME.'
    if ($errors.Count -gt 0) {
        $message = "$message Tried: $($errors -join '; ')"
    }
    throw $message
}
