function Get-RustyXrProjectionPropertyHygieneKeys {
    return @(
        "debug.rustyxr.camera.projection.mode",
        "debug.rustyxr.projection.geometry.profile",
        "debug.rustyxr.projection.scale",
        "debug.rustyxr.projection.depth.meters",
        "debug.rustyxr.camera.projection.fov.y.degrees",
        "debug.rustyxr.camera.preview.fov.y.degrees",
        "debug.rustyxr.camera.preview.offset.y.meters",
        "debug.rustyxr.camera.raw.overlay.overscan",
        "debug.rustyxr.projection.area.scale.uv",
        "debug.rustyxr.projection.area.scale.x",
        "debug.rustyxr.projection.area.scale.y",
        "debug.rustyxr.projection.area.offset.x.uv",
        "debug.rustyxr.projection.area.offset.y.uv",
        "debug.rustyxr.projection.area.left.offset.x.uv",
        "debug.rustyxr.projection.area.left.offset.y.uv",
        "debug.rustyxr.projection.area.right.offset.x.uv",
        "debug.rustyxr.projection.area.right.offset.y.uv",
        "debug.rustyxr.projection.area.radius.x.uv",
        "debug.rustyxr.projection.area.radius.y.uv",
        "debug.rustyxr.projection.area.corner.radius.uv",
        "debug.rustyxr.projection.area.opacity",
        "debug.rustyxr.projection.border.opacity",
        "debug.rustyxr.projection.border.policy",
        "debug.rustyxr.projection.alpha.mode",
        "debug.rustyxr.projection.alpha.scale",
        "debug.rustyxr.projection.alpha.bias",
        "debug.rustyxr.source.eye.mapping",
        "debug.rustyxr.source.texture.transform.source",
        "debug.rustyxr.source.visible.rect.x.uv",
        "debug.rustyxr.source.visible.rect.y.uv",
        "debug.rustyxr.source.visible.rect.width.uv",
        "debug.rustyxr.source.visible.rect.height.uv",
        "debug.rustyxr.oes.projection.runtime.resolution.enabled",
        "debug.rustyxr.xr.render.scale",
        "debug.rustyxr.xr.display.refresh.rate.hz",
        "debug.rustyxr.makepad.camera.projection.geometry.profile",
        "debug.rustyxr.makepad.broker.h264.enabled",
        "debug.rustyxr.makepad.broker.h264.host",
        "debug.rustyxr.makepad.broker.h264.broker.port",
        "debug.rustyxr.makepad.broker.h264.stream.port",
        "debug.rustyxr.makepad.broker.h264.right.stream.port",
        "debug.rustyxr.makepad.broker.h264.source.mode",
        "debug.rustyxr.makepad.broker.h264.synthetic.pattern",
        "debug.rustyxr.makepad.broker.h264.projection.geometry.profile",
        "debug.rustyxr.makepad.broker.h264.synthetic.projection.profile",
        "debug.rustyxr.makepad.broker.h264.left.camera.id",
        "debug.rustyxr.makepad.broker.h264.right.camera.id",
        "debug.rustyxr.makepad.broker.h264.width",
        "debug.rustyxr.makepad.broker.h264.height",
        "debug.rustyxr.makepad.broker.h264.capture.ms",
        "debug.rustyxr.makepad.broker.h264.max.packets",
        "debug.rustyxr.makepad.broker.h264.bitrate.bps",
        "debug.rustyxr.makepad.broker.h264.frame.rate.hz",
        "debug.rustyxr.makepad.broker.h264.stream.timeout.ms",
        "debug.rustyxr.makepad.broker.h264.decode.timeout.ms",
        "debug.rustyxr.makepad.broker.h264.live.stream",
        "debug.rustyxr.makepad.projection.border.policy",
        "debug.rustyxr.makepad.native.passthrough.enabled",
        "debug.rustyxr.makepad.projection.border.opacity",
        "debug.rustyxr.makepad.projection.area.opacity",
        "debug.rustyxr.makepad.projection.alpha.mode",
        "debug.rustyxr.makepad.projection.alpha.scale",
        "debug.rustyxr.makepad.projection.alpha.bias",
        "debug.rustyxr.makepad.projection.runtime.resolution.enabled",
        "debug.rustyxr.makepad.processing.layer",
        "debug.rustyxr.makepad.blur.radius.px",
        "debug.rustyxr.makepad.direct.camera.hardware.buffer.external",
        "debug.rustyxr.makepad.horizontal.alignment.strength",
        "debug.rustyxr.makepad.horizontal.offset.uv",
        "debug.rustyxr.makepad.horizontal.offset.left.uv",
        "debug.rustyxr.makepad.horizontal.offset.right.uv",
        "debug.rustyxr.makepad.vertical.offset.uv",
        "debug.rustyxr.makepad.content.uv.scale",
        "debug.rustyxr.makepad.projection.area.diagnostic",
        "debug.rustyxr.makepad.projection.area.offset.left.uv",
        "debug.rustyxr.makepad.projection.area.offset.right.uv",
        "debug.rustyxr.makepad.projection.area.offset.vertical.uv",
        "debug.rustyxr.makepad.projection.area.scale.x",
        "debug.rustyxr.makepad.projection.area.scale.y",
        "debug.rustyxr.makepad.projection.area.radius.x.uv",
        "debug.rustyxr.makepad.projection.area.radius.y.uv",
        "debug.rustyxr.makepad.projection.area.corner.radius.uv",
        "debug.rustyxr.makepad.projection.area.keystone.x",
        "debug.rustyxr.makepad.projection.area.bow.x"
    )
}

function Invoke-RustyXrProjectionPropertyHygieneAdb {
    param(
        [string]$Adb,
        [string]$Serial,
        [string[]]$Arguments
    )
    $adbArguments = @()
    if ($Serial) {
        $adbArguments += @("-s", $Serial)
    }
    $adbArguments += $Arguments
    & $Adb @adbArguments
}

function Read-RustyXrProjectionPropertyValues {
    param(
        [string]$Adb,
        [string]$Serial,
        [string[]]$Keys
    )
    foreach ($key in $Keys) {
        $value = (Invoke-RustyXrProjectionPropertyHygieneAdb -Adb $Adb -Serial $Serial -Arguments @("shell", "getprop", $key)) -join ""
        $trimmed = $value.Trim()
        [ordered]@{
            property = $key
            value = $trimmed
            nonEmpty = $trimmed.Length -gt 0
        }
    }
}

function Invoke-RustyXrProjectionPropertyHygiene {
    param(
        [string]$Adb = "adb",
        [string]$Serial = "",
        [ValidateSet("fail", "clear", "ignore")]
        [string]$Mode = "fail",
        [string]$OutputPath = "",
        [string[]]$Keys = @()
    )
    $keysToCheck = if ($Keys.Count -gt 0) {
        @($Keys | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
    } else {
        @(Get-RustyXrProjectionPropertyHygieneKeys | Sort-Object -Unique)
    }

    $before = @(Read-RustyXrProjectionPropertyValues -Adb $Adb -Serial $Serial -Keys $keysToCheck)
    $staleBefore = @($before | Where-Object { $_.nonEmpty })
    $cleared = @()
    if ($Mode -eq "clear") {
        foreach ($entry in $staleBefore) {
            if ($entry.property -notmatch '^[A-Za-z0-9_.-]+$') {
                throw "Refusing to clear invalid Android property name '$($entry.property)'."
            }
            # Android setprop requires a non-empty VALUE argument. A single
            # space is accepted by setprop and is trimmed to empty by the
            # hygiene reader and runtime property parsers.
            Invoke-RustyXrProjectionPropertyHygieneAdb -Adb $Adb -Serial $Serial -Arguments @("shell", "setprop $($entry.property) ' '") | Out-Null
            $cleared += $entry.property
        }
    }
    $after = @(Read-RustyXrProjectionPropertyValues -Adb $Adb -Serial $Serial -Keys $keysToCheck)
    $afterNonEmpty = @($after | Where-Object { $_.nonEmpty })

    $status = "ok"
    if ($Mode -eq "fail" -and $staleBefore.Count -gt 0) {
        $status = "failed"
    }
    if ($Mode -eq "clear" -and $afterNonEmpty.Count -gt 0) {
        $status = "failed"
    }

    $summary = [ordered]@{
        schemaVersion = "rusty.xr.projection-property-hygiene.v1"
        checkedAt = (Get-Date).ToString("o")
        mode = $Mode
        keyCount = $keysToCheck.Count
        staleBeforeCount = $staleBefore.Count
        staleBefore = $staleBefore
        clearedCount = $cleared.Count
        clearedProperties = $cleared
        afterNonEmptyCount = $afterNonEmpty.Count
        afterNonEmpty = $afterNonEmpty
        status = $status
    }

    if ($OutputPath) {
        $directory = Split-Path -Path $OutputPath -Parent
        if ($directory) {
            New-Item -ItemType Directory -Force -Path $directory | Out-Null
        }
        $summary | ConvertTo-Json -Depth 5 | Set-Content -Path $OutputPath -Encoding UTF8
    }

    if ($Mode -eq "fail" -and $staleBefore.Count -gt 0) {
        throw "Stale debug.rustyxr projection properties are present; see $OutputPath or rerun with property hygiene mode clear."
    }
    if ($Mode -eq "clear" -and $afterNonEmpty.Count -gt 0) {
        throw "Failed to clear all debug.rustyxr projection properties; see $OutputPath."
    }

    return $summary
}
