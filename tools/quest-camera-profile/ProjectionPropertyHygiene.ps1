function Get-RustyQuestMakepadProjectionPropertyHygieneKeys {
    # Legacy Makepad property names are retained here only as stale-device cleanup targets.
    $legacyMakepadCleanupKeys = @(
        "debug.rustyquest.makepad.camera.projection.mode",
        "debug.rustyquest.makepad.camera.projection.geometry.profile",
        "debug.rustyquest.makepad.camera.source.sampling.mode",
        "debug.rustyquest.makepad.camera.target.screen.uv.rect",
        "debug.rustyquest.makepad.camera.left.target.screen.uv.rect",
        "debug.rustyquest.makepad.camera.right.target.screen.uv.rect",
        "debug.rustyquest.makepad.projection.geometry.profile",
        "debug.rustyquest.makepad.projection.scale",
        "debug.rustyquest.makepad.projection.depth.meters",
        "debug.rustyquest.makepad.camera.projection.fov.y.degrees",
        "debug.rustyquest.makepad.camera.preview.fov.y.degrees",
        "debug.rustyquest.makepad.camera.preview.offset.y.meters",
        "debug.rustyquest.makepad.camera.raw.overlay.overscan",
        "debug.rustyquest.makepad.projection.area.scale.uv",
        "debug.rustyquest.makepad.projection.area.scale.x",
        "debug.rustyquest.makepad.projection.area.scale.y",
        "debug.rustyquest.makepad.projection.area.offset.x.uv",
        "debug.rustyquest.makepad.projection.area.offset.y.uv",
        "debug.rustyquest.makepad.projection.area.left.offset.x.uv",
        "debug.rustyquest.makepad.projection.area.left.offset.y.uv",
        "debug.rustyquest.makepad.projection.area.right.offset.x.uv",
        "debug.rustyquest.makepad.projection.area.right.offset.y.uv",
        "debug.rustyquest.makepad.projection.area.radius.x.uv",
        "debug.rustyquest.makepad.projection.area.radius.y.uv",
        "debug.rustyquest.makepad.projection.area.corner.radius.uv",
        "debug.rustyquest.makepad.projection.area.opacity",
        "debug.rustyquest.makepad.projection.border.opacity",
        "debug.rustyquest.makepad.projection.border.policy",
        "debug.rustyquest.makepad.projection.target.offset.x.uv",
        "debug.rustyquest.makepad.projection.target.offset.y.uv",
        "debug.rustyquest.makepad.projection.target.scale",
        "debug.rustyquest.makepad.projection.target.joystick.controls",
        "debug.rustyquest.makepad.processing.layer",
        "debug.rustyquest.makepad.camera.blur.radius.px",
        "debug.rustyquest.makepad.peripheral.stretch.mode",
        "debug.rustyquest.makepad.peripheral.stretch.core.scale",
        "debug.rustyquest.makepad.peripheral.stretch.edge.inset.uv",
        "debug.rustyquest.makepad.peripheral.stretch.max.inset.uv",
        "debug.rustyquest.makepad.peripheral.stretch.curve",
        "debug.rustyquest.makepad.peripheral.stretch.inner.blend.uv",
        "debug.rustyquest.makepad.peripheral.stretch.blend.curve",
        "debug.rustyquest.makepad.peripheral.stretch.blend.mode",
        "debug.rustyquest.makepad.peripheral.stretch.corner.mode",
        "debug.rustyquest.makepad.peripheral.stretch.debug",
        "debug.rustyquest.makepad.projection.alpha.mode",
        "debug.rustyquest.makepad.projection.alpha.scale",
        "debug.rustyquest.makepad.projection.alpha.bias",
        "debug.rustyquest.makepad.source.eye.mapping",
        "debug.rustyquest.makepad.source.texture.transform.source",
        "debug.rustyquest.makepad.source.visible.rect.x.uv",
        "debug.rustyquest.makepad.source.visible.rect.y.uv",
        "debug.rustyquest.makepad.source.visible.rect.width.uv",
        "debug.rustyquest.makepad.source.visible.rect.height.uv",
        "debug.rustyquest.makepad.oes.projection.runtime.resolution.enabled",
        "debug.rustyquest.makepad.xr.render.scale",
        "debug.rustyquest.makepad.xr.display.refresh.rate.hz",
        "debug.rustyquest.makepad.broker.h264.enabled",
        "debug.rustyquest.makepad.broker.h264.host",
        "debug.rustyquest.makepad.broker.h264.broker.port",
        "debug.rustyquest.makepad.broker.h264.stream.port",
        "debug.rustyquest.makepad.broker.h264.right.stream.port",
        "debug.rustyquest.makepad.broker.h264.source.mode",
        "debug.rustyquest.makepad.broker.h264.synthetic.pattern",
        "debug.rustyquest.makepad.broker.h264.projection.geometry.profile",
        "debug.rustyquest.makepad.broker.h264.synthetic.projection.profile",
        "debug.rustyquest.makepad.broker.h264.left.camera.id",
        "debug.rustyquest.makepad.broker.h264.right.camera.id",
        "debug.rustyquest.makepad.broker.h264.width",
        "debug.rustyquest.makepad.broker.h264.height",
        "debug.rustyquest.makepad.broker.h264.capture.ms",
        "debug.rustyquest.makepad.broker.h264.max.packets",
        "debug.rustyquest.makepad.broker.h264.bitrate.bps",
        "debug.rustyquest.makepad.broker.h264.frame.rate.hz",
        "debug.rustyquest.makepad.broker.h264.stream.timeout.ms",
        "debug.rustyquest.makepad.broker.h264.decode.timeout.ms",
        "debug.rustyquest.makepad.broker.h264.live.stream",
        "debug.rustyquest.makepad.native.passthrough.enabled",
        "debug.rustyquest.makepad.projection.runtime.resolution.enabled",
        "debug.rustyquest.makepad.projection.sample.mode",
        "debug.rustyquest.makepad.direct.camera.hardware.buffer.external",
        "debug.rustyquest.makepad.horizontal.alignment.strength",
        "debug.rustyquest.makepad.horizontal.offset.uv",
        "debug.rustyquest.makepad.horizontal.offset.left.uv",
        "debug.rustyquest.makepad.horizontal.offset.right.uv",
        "debug.rustyquest.makepad.vertical.offset.uv",
        "debug.rustyquest.makepad.content.uv.scale",
        "debug.rustyquest.makepad.projection.area.diagnostic",
        "debug.rustyquest.makepad.projection.area.offset.left.uv",
        "debug.rustyquest.makepad.projection.area.offset.right.uv",
        "debug.rustyquest.makepad.projection.area.offset.vertical.uv",
        "debug.rustyquest.makepad.projection.area.keystone.x",
        "debug.rustyquest.makepad.projection.area.bow.x",
        "debug.rustyquest.makepad.mesh.replay.enabled",
        "debug.rustyquest.makepad.mesh.replay.source",
        "debug.rustyquest.makepad.mesh.replay.speed",
        "debug.rustyquest.makepad.mesh.replay.opacity"
    )
    $sharedQuestProjectionKeys = @(
        "debug.rustyquest.camera.projection.mode",
        "debug.rustyquest.projection.geometry.profile",
        "debug.rustyquest.projection.scale",
        "debug.rustyquest.projection.depth.meters",
        "debug.rustyquest.camera.projection.fov.y.degrees",
        "debug.rustyquest.camera.preview.fov.y.degrees",
        "debug.rustyquest.camera.preview.offset.y.meters",
        "debug.rustyquest.camera.raw.overlay.overscan",
        "debug.rustyquest.projection.area.scale.uv",
        "debug.rustyquest.projection.area.scale.x",
        "debug.rustyquest.projection.area.scale.y",
        "debug.rustyquest.projection.area.offset.x.uv",
        "debug.rustyquest.projection.area.offset.y.uv",
        "debug.rustyquest.projection.area.left.offset.x.uv",
        "debug.rustyquest.projection.area.left.offset.y.uv",
        "debug.rustyquest.projection.area.right.offset.x.uv",
        "debug.rustyquest.projection.area.right.offset.y.uv",
        "debug.rustyquest.projection.area.radius.x.uv",
        "debug.rustyquest.projection.area.radius.y.uv",
        "debug.rustyquest.projection.area.corner.radius.uv",
        "debug.rustyquest.projection.area.opacity",
        "debug.rustyquest.projection.border.opacity",
        "debug.rustyquest.projection.border.policy",
        "debug.rustyquest.projection.target.offset.x.uv",
        "debug.rustyquest.projection.target.offset.y.uv",
        "debug.rustyquest.projection.target.scale",
        "debug.rustyquest.projection.target.joystick.controls",
        "debug.rustyquest.projection.target.breath.controls",
        "debug.rustyquest.projection.target.breath.stream",
        "debug.rustyquest.projection.target.breath.min.scale",
        "debug.rustyquest.projection.target.breath.max.scale",
        "debug.rustyquest.projection.target.breath.smoothing.alpha",
        "debug.rustyquest.projection.target.breath.invert",
        "debug.rustyquest.projection.target.breath.min.quality",
        "debug.rustyquest.processing.layer",
        "debug.rustyquest.camera.blur.radius.px",
        "debug.rustyquest.peripheral.stretch.mode",
        "debug.rustyquest.peripheral.stretch.core.scale",
        "debug.rustyquest.peripheral.stretch.edge.inset.uv",
        "debug.rustyquest.peripheral.stretch.max.inset.uv",
        "debug.rustyquest.peripheral.stretch.curve",
        "debug.rustyquest.peripheral.stretch.inner.blend.uv",
        "debug.rustyquest.peripheral.stretch.blend.curve",
        "debug.rustyquest.peripheral.stretch.blend.mode",
        "debug.rustyquest.peripheral.stretch.corner.mode",
        "debug.rustyquest.peripheral.stretch.debug",
        "debug.rustyquest.projection.alpha.mode",
        "debug.rustyquest.projection.alpha.scale",
        "debug.rustyquest.projection.alpha.bias",
        "debug.rustyquest.source.eye.mapping",
        "debug.rustyquest.source.texture.rotation",
        "debug.rustyquest.source.texture.flip.x",
        "debug.rustyquest.source.texture.flip.y",
        "debug.rustyquest.source.texture.mirror",
        "debug.rustyquest.source.texture.transform.source",
        "debug.rustyquest.source.texture.transform.reason",
        "debug.rustyquest.source.left.texture.transform.source",
        "debug.rustyquest.source.right.texture.transform.source",
        "debug.rustyquest.source.visible.rect.x.uv",
        "debug.rustyquest.source.visible.rect.y.uv",
        "debug.rustyquest.source.visible.rect.width.uv",
        "debug.rustyquest.source.visible.rect.height.uv",
        "debug.rustyquest.oes.projection.runtime.resolution.enabled"
    )
    return @($legacyMakepadCleanupKeys + $sharedQuestProjectionKeys)
}

function Invoke-RustyQuestMakepadProjectionPropertyHygieneAdb {
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

function Read-RustyQuestMakepadProjectionPropertyValues {
    param(
        [string]$Adb,
        [string]$Serial,
        [string[]]$Keys
    )
    foreach ($key in $Keys) {
        $value = (Invoke-RustyQuestMakepadProjectionPropertyHygieneAdb -Adb $Adb -Serial $Serial -Arguments @("shell", "getprop", $key)) -join ""
        $trimmed = $value.Trim()
        [ordered]@{
            property = $key
            value = $trimmed
            nonEmpty = $trimmed.Length -gt 0
        }
    }
}

function Invoke-RustyQuestMakepadProjectionPropertyHygiene {
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
        @(Get-RustyQuestMakepadProjectionPropertyHygieneKeys | Sort-Object -Unique)
    }

    $before = @(Read-RustyQuestMakepadProjectionPropertyValues -Adb $Adb -Serial $Serial -Keys $keysToCheck)
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
            Invoke-RustyQuestMakepadProjectionPropertyHygieneAdb -Adb $Adb -Serial $Serial -Arguments @("shell", "setprop $($entry.property) ' '") | Out-Null
            $cleared += $entry.property
        }
    }
    $after = @(Read-RustyQuestMakepadProjectionPropertyValues -Adb $Adb -Serial $Serial -Keys $keysToCheck)
    $afterNonEmpty = @($after | Where-Object { $_.nonEmpty })

    $status = "ok"
    if ($Mode -eq "fail" -and $staleBefore.Count -gt 0) {
        $status = "failed"
    }
    if ($Mode -eq "clear" -and $afterNonEmpty.Count -gt 0) {
        $status = "failed"
    }

    $summary = [ordered]@{
        schemaVersion = "rusty.quest.makepad.projection-property-hygiene.v1"
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
        throw "Stale debug.rustyquest.makepad projection properties are present; see $OutputPath or rerun with property hygiene mode clear."
    }
    if ($Mode -eq "clear" -and $afterNonEmpty.Count -gt 0) {
        throw "Failed to clear all debug.rustyquest.makepad projection properties; see $OutputPath."
    }

    return $summary
}

