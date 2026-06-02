param(
    [string]$Serial = "",
    [double]$Strength = [double]::NaN,
    [double]$GlobalUv = [double]::NaN,
    [double]$LeftUv = [double]::NaN,
    [double]$RightUv = [double]::NaN,
    [double]$VerticalUv = [double]::NaN,
    [double]$SymmetricUv = [double]::NaN,
    [double]$ContentScale = [double]::NaN,
    [double]$ProjectionBorderOpacity = [double]::NaN,
    [ValidateSet("solid-red", "passthrough-underlay")]
    [string]$ProjectionBorderPolicy = "",
    [ValidateSet("", "raw", "blur", "peripheral-stretch")]
    [string]$ProcessingLayer = "",
    [ValidateSet("", "camera", "solid-color", "solid-no-texture", "clear-only")]
    [string]$ProjectionSampleMode = "",
    [double]$BlurRadiusPx = [double]::NaN,
    [ValidateSet("", "edge-stretch")]
    [string]$PeripheralStretchMode = "",
    [double]$PeripheralStretchCoreScale = [double]::NaN,
    [double]$PeripheralStretchEdgeInsetUv = [double]::NaN,
    [double]$PeripheralStretchMaxInsetUv = [double]::NaN,
    [double]$PeripheralStretchCurve = [double]::NaN,
    [double]$PeripheralStretchInnerBlendUv = [double]::NaN,
    [double]$PeripheralStretchBlendCurve = [double]::NaN,
    [ValidateSet("", "off", "target-inner-band")]
    [string]$PeripheralStretchBlendMode = "",
    [ValidateSet("", "target-footprint")]
    [string]$PeripheralStretchCornerMode = "",
    [ValidateSet("", "off", "regions", "sample-uv")]
    [string]$PeripheralStretchDebug = "",
    [double]$ProjectionDepthMeters = [double]::NaN,
    [double]$ProjectionAreaDiagnostic = [double]::NaN,
    [double]$ProjectionAreaLeftUv = [double]::NaN,
    [double]$ProjectionAreaRightUv = [double]::NaN,
    [double]$ProjectionAreaVerticalUv = [double]::NaN,
    [double]$ProjectionAreaScaleX = [double]::NaN,
    [double]$ProjectionAreaScaleY = [double]::NaN,
    [double]$ProjectionTargetOffsetXUv = [double]::NaN,
    [double]$ProjectionTargetOffsetYUv = [double]::NaN,
    [double]$ProjectionTargetScale = [double]::NaN,
    [ValidateSet("", "off", "offset-scale")]
    [string]$ProjectionTargetJoystickControls = "",
    [double]$ProjectionAreaRadiusXUv = [double]::NaN,
    [double]$ProjectionAreaRadiusYUv = [double]::NaN,
    [double]$ProjectionAreaCornerRadiusUv = [double]::NaN,
    [double]$ProjectionAreaKeystoneX = [double]::NaN,
    [double]$ProjectionAreaBowX = [double]::NaN,
    [ValidateSet("", "fixed", "red", "green", "blue", "luma", "inverse-red", "inverse-green", "inverse-blue", "inverse-luma", "red-dominance", "green-dominance", "blue-dominance", "saturation", "inverse-saturation")]
    [string]$ProjectionAlphaMode = "",
    [double]$ProjectionAlphaScale = [double]::NaN,
    [double]$ProjectionAlphaBias = [double]::NaN,
    [ValidateSet("", "true", "false")]
    [string]$UseResolvedProjectionRuntime = "true",
    [switch]$Reset
)

$ErrorActionPreference = "Stop"

$ProjectionAreaScaleMin = 0.01
$ProjectionAreaScaleMax = 10.0

function Invoke-Adb {
    param([string[]]$Arguments)
    $adbArgs = @()
    if ($Serial) {
        $adbArgs += @("-s", $Serial)
    }
    $adbArgs += $Arguments
    & adb @adbArgs
    if ($LASTEXITCODE -ne 0) {
        throw "adb failed with exit code $LASTEXITCODE"
    }
}

function Assert-Range {
    param(
        [string]$Name,
        [double]$Value,
        [double]$Min,
        [double]$Max
    )
    if ([double]::IsNaN($Value)) {
        return
    }
    if ([double]::IsInfinity($Value) -or $Value -lt $Min -or $Value -gt $Max) {
        throw "$Name must be finite and within [$Min, $Max]; got $Value"
    }
}

function Set-Prop {
    param(
        [string]$Name,
        [double]$Value
    )
    $text = $Value.ToString("0.######", [Globalization.CultureInfo]::InvariantCulture)
    Invoke-Adb -Arguments @("shell", "setprop", $Name, $text)
}

$properties = [ordered]@{
    Strength = "debug.rustyxr.makepad.horizontal.alignment.strength"
    GlobalUv = "debug.rustyxr.makepad.horizontal.offset.uv"
    LeftUv = "debug.rustyxr.makepad.horizontal.offset.left.uv"
    RightUv = "debug.rustyxr.makepad.horizontal.offset.right.uv"
    VerticalUv = "debug.rustyxr.makepad.vertical.offset.uv"
    ContentScale = "debug.rustyxr.makepad.content.uv.scale"
    ProjectionBorderOpacity = "debug.rustyxr.projection.border.opacity"
    ProjectionBorderPolicy = "debug.rustyxr.projection.border.policy"
    NativePassthroughEnabled = "debug.rustyxr.makepad.native.passthrough.enabled"
    ProcessingLayer = "debug.rustyxr.processing.layer"
    ProcessingLayerLegacy = "debug.rustyxr.makepad.processing.layer"
    ProjectionSampleMode = "debug.rustyxr.makepad.projection.sample.mode"
    BlurRadiusPx = "debug.rustyxr.camera.blur.radius.px"
    BlurRadiusPxLegacy = "debug.rustyxr.makepad.blur.radius.px"
    PeripheralStretchMode = "debug.rustyxr.peripheral.stretch.mode"
    PeripheralStretchCoreScale = "debug.rustyxr.peripheral.stretch.core.scale"
    PeripheralStretchEdgeInsetUv = "debug.rustyxr.peripheral.stretch.edge.inset.uv"
    PeripheralStretchMaxInsetUv = "debug.rustyxr.peripheral.stretch.max.inset.uv"
    PeripheralStretchCurve = "debug.rustyxr.peripheral.stretch.curve"
    PeripheralStretchInnerBlendUv = "debug.rustyxr.peripheral.stretch.inner.blend.uv"
    PeripheralStretchBlendCurve = "debug.rustyxr.peripheral.stretch.blend.curve"
    PeripheralStretchBlendMode = "debug.rustyxr.peripheral.stretch.blend.mode"
    PeripheralStretchCornerMode = "debug.rustyxr.peripheral.stretch.corner.mode"
    PeripheralStretchDebug = "debug.rustyxr.peripheral.stretch.debug"
    ProjectionDepthMeters = "debug.rustyxr.projection.depth.meters"
    ProjectionAreaDiagnostic = "debug.rustyxr.makepad.projection.area.diagnostic"
    ProjectionAreaLeftOffsetXUv = "debug.rustyxr.projection.area.left.offset.x.uv"
    ProjectionAreaRightOffsetXUv = "debug.rustyxr.projection.area.right.offset.x.uv"
    ProjectionAreaOffsetYUv = "debug.rustyxr.projection.area.offset.y.uv"
    ProjectionAreaScaleX = "debug.rustyxr.projection.area.scale.x"
    ProjectionAreaScaleY = "debug.rustyxr.projection.area.scale.y"
    ProjectionTargetOffsetXUv = "debug.rustyxr.projection.target.offset.x.uv"
    ProjectionTargetOffsetYUv = "debug.rustyxr.projection.target.offset.y.uv"
    ProjectionTargetScale = "debug.rustyxr.projection.target.scale"
    ProjectionTargetJoystickControls = "debug.rustyxr.projection.target.joystick.controls"
    ProjectionTargetJoystickControlsLegacy = "debug.rustyxr.makepad.projection.target.joystick.controls"
    ProjectionAreaRadiusXUv = "debug.rustyxr.projection.area.radius.x.uv"
    ProjectionAreaRadiusYUv = "debug.rustyxr.projection.area.radius.y.uv"
    ProjectionAreaCornerRadiusUv = "debug.rustyxr.projection.area.corner.radius.uv"
    ProjectionAreaKeystoneX = "debug.rustyxr.makepad.projection.area.keystone.x"
    ProjectionAreaBowX = "debug.rustyxr.makepad.projection.area.bow.x"
    ProjectionAlphaMode = "debug.rustyxr.projection.alpha.mode"
    ProjectionAlphaScale = "debug.rustyxr.projection.alpha.scale"
    ProjectionAlphaBias = "debug.rustyxr.projection.alpha.bias"
    ResolvedProjectionRuntime = "debug.rustyxr.makepad.projection.runtime.resolution.enabled"
}

if ($Reset) {
    $Strength = 0.0
    $GlobalUv = 0.0
    $LeftUv = 0.0
    $RightUv = 0.0
    $VerticalUv = 0.0
    $ContentScale = 1.60
    $ProjectionBorderOpacity = 1.0
    $ProjectionBorderPolicy = "solid-red"
    $ProcessingLayer = "raw"
    $BlurRadiusPx = 2.0
    $PeripheralStretchMode = "edge-stretch"
    $PeripheralStretchCoreScale = 1.0
    $PeripheralStretchEdgeInsetUv = 0.015
    $PeripheralStretchMaxInsetUv = 0.14
    $PeripheralStretchCurve = 1.6
    $PeripheralStretchInnerBlendUv = 0.040
    $PeripheralStretchBlendCurve = 1.6
    $PeripheralStretchBlendMode = "target-inner-band"
    $PeripheralStretchCornerMode = "target-footprint"
    $PeripheralStretchDebug = "off"
    $ProjectionDepthMeters = 1.0
    $ProjectionAreaDiagnostic = 0.0
    $ProjectionAreaLeftUv = 0.0
    $ProjectionAreaRightUv = 0.0
    $ProjectionAreaVerticalUv = 0.0
    $ProjectionAreaScaleX = 1.0
    $ProjectionAreaScaleY = 1.0
    $ProjectionTargetOffsetXUv = 0.0
    $ProjectionTargetOffsetYUv = 0.0
    $ProjectionTargetScale = 1.0
    $ProjectionTargetJoystickControls = "offset-scale"
    $ProjectionAreaRadiusXUv = 0.5
    $ProjectionAreaRadiusYUv = 0.5
    $ProjectionAreaCornerRadiusUv = 0.0
    $ProjectionAreaKeystoneX = 0.0
    $ProjectionAreaBowX = 0.0
    $ProjectionAlphaMode = "fixed"
    $ProjectionAlphaScale = 1.0
    $ProjectionAlphaBias = 0.0
    $UseResolvedProjectionRuntime = "true"
}

if (-not [double]::IsNaN($SymmetricUv)) {
    $LeftUv = $SymmetricUv
    $RightUv = -$SymmetricUv
}

Assert-Range -Name "Strength" -Value $Strength -Min -4.0 -Max 4.0
Assert-Range -Name "GlobalUv" -Value $GlobalUv -Min -0.5 -Max 0.5
Assert-Range -Name "LeftUv" -Value $LeftUv -Min -0.5 -Max 0.5
Assert-Range -Name "RightUv" -Value $RightUv -Min -0.5 -Max 0.5
Assert-Range -Name "VerticalUv" -Value $VerticalUv -Min -0.5 -Max 0.5
Assert-Range -Name "SymmetricUv" -Value $SymmetricUv -Min -0.5 -Max 0.5
Assert-Range -Name "ContentScale" -Value $ContentScale -Min 1.0 -Max 2.4
Assert-Range -Name "ProjectionBorderOpacity" -Value $ProjectionBorderOpacity -Min 0.0 -Max 1.0
Assert-Range -Name "BlurRadiusPx" -Value $BlurRadiusPx -Min 0.0 -Max 16.0
Assert-Range -Name "PeripheralStretchCoreScale" -Value $PeripheralStretchCoreScale -Min 0.05 -Max 1.0
Assert-Range -Name "PeripheralStretchEdgeInsetUv" -Value $PeripheralStretchEdgeInsetUv -Min 0.0 -Max 0.49
Assert-Range -Name "PeripheralStretchMaxInsetUv" -Value $PeripheralStretchMaxInsetUv -Min 0.0 -Max 0.49
Assert-Range -Name "PeripheralStretchCurve" -Value $PeripheralStretchCurve -Min 0.25 -Max 6.0
Assert-Range -Name "PeripheralStretchInnerBlendUv" -Value $PeripheralStretchInnerBlendUv -Min 0.0 -Max 0.25
Assert-Range -Name "PeripheralStretchBlendCurve" -Value $PeripheralStretchBlendCurve -Min 0.25 -Max 6.0
Assert-Range -Name "ProjectionDepthMeters" -Value $ProjectionDepthMeters -Min 0.05 -Max 10.0
Assert-Range -Name "ProjectionAreaDiagnostic" -Value $ProjectionAreaDiagnostic -Min 0.0 -Max 2.0
Assert-Range -Name "ProjectionAreaLeftUv" -Value $ProjectionAreaLeftUv -Min -0.5 -Max 0.5
Assert-Range -Name "ProjectionAreaRightUv" -Value $ProjectionAreaRightUv -Min -0.5 -Max 0.5
Assert-Range -Name "ProjectionAreaVerticalUv" -Value $ProjectionAreaVerticalUv -Min -0.5 -Max 0.5
Assert-Range -Name "ProjectionAreaScaleX" -Value $ProjectionAreaScaleX -Min $ProjectionAreaScaleMin -Max $ProjectionAreaScaleMax
Assert-Range -Name "ProjectionAreaScaleY" -Value $ProjectionAreaScaleY -Min $ProjectionAreaScaleMin -Max $ProjectionAreaScaleMax
Assert-Range -Name "ProjectionTargetOffsetXUv" -Value $ProjectionTargetOffsetXUv -Min -0.5 -Max 0.5
Assert-Range -Name "ProjectionTargetOffsetYUv" -Value $ProjectionTargetOffsetYUv -Min -0.5 -Max 0.5
Assert-Range -Name "ProjectionTargetScale" -Value $ProjectionTargetScale -Min 0.05 -Max 1.50
Assert-Range -Name "ProjectionAreaRadiusXUv" -Value $ProjectionAreaRadiusXUv -Min 0.05 -Max 0.5
Assert-Range -Name "ProjectionAreaRadiusYUv" -Value $ProjectionAreaRadiusYUv -Min 0.05 -Max 0.5
Assert-Range -Name "ProjectionAreaCornerRadiusUv" -Value $ProjectionAreaCornerRadiusUv -Min 0.0 -Max 0.5
Assert-Range -Name "ProjectionAreaKeystoneX" -Value $ProjectionAreaKeystoneX -Min -0.45 -Max 0.45
Assert-Range -Name "ProjectionAreaBowX" -Value $ProjectionAreaBowX -Min -0.25 -Max 0.25
Assert-Range -Name "ProjectionAlphaScale" -Value $ProjectionAlphaScale -Min 0.0 -Max 4.0
Assert-Range -Name "ProjectionAlphaBias" -Value $ProjectionAlphaBias -Min -1.0 -Max 1.0

if (-not [double]::IsNaN($Strength)) {
    Set-Prop -Name $properties.Strength -Value $Strength
}
if (-not [double]::IsNaN($GlobalUv)) {
    Set-Prop -Name $properties.GlobalUv -Value $GlobalUv
}
if (-not [double]::IsNaN($LeftUv)) {
    Set-Prop -Name $properties.LeftUv -Value $LeftUv
}
if (-not [double]::IsNaN($RightUv)) {
    Set-Prop -Name $properties.RightUv -Value $RightUv
}
if (-not [double]::IsNaN($VerticalUv)) {
    Set-Prop -Name $properties.VerticalUv -Value $VerticalUv
}
if (-not [double]::IsNaN($ContentScale)) {
    Set-Prop -Name $properties.ContentScale -Value $ContentScale
}
if (-not [double]::IsNaN($ProjectionBorderOpacity)) {
    Set-Prop -Name $properties.ProjectionBorderOpacity -Value $ProjectionBorderOpacity
}
if ($ProjectionBorderPolicy) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionBorderPolicy, $ProjectionBorderPolicy)
    $nativePassthrough = if ($ProjectionBorderPolicy -eq "passthrough-underlay") { "true" } else { "false" }
    Invoke-Adb -Arguments @("shell", "setprop", $properties.NativePassthroughEnabled, $nativePassthrough)
}
if ($ProjectionAlphaMode) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionAlphaMode, $ProjectionAlphaMode)
    if ($ProjectionAlphaMode -ne "fixed") {
        Invoke-Adb -Arguments @("shell", "setprop", $properties.NativePassthroughEnabled, "true")
    }
}
if ($ProcessingLayer) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProcessingLayer, $ProcessingLayer)
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProcessingLayerLegacy, $ProcessingLayer)
}
if ($ProjectionSampleMode) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionSampleMode, $ProjectionSampleMode)
}
if (-not [double]::IsNaN($BlurRadiusPx)) {
    Set-Prop -Name $properties.BlurRadiusPx -Value $BlurRadiusPx
    Set-Prop -Name $properties.BlurRadiusPxLegacy -Value $BlurRadiusPx
}
if ($PeripheralStretchMode) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.PeripheralStretchMode, $PeripheralStretchMode)
}
if (-not [double]::IsNaN($PeripheralStretchCoreScale)) {
    Set-Prop -Name $properties.PeripheralStretchCoreScale -Value $PeripheralStretchCoreScale
}
if (-not [double]::IsNaN($PeripheralStretchEdgeInsetUv)) {
    Set-Prop -Name $properties.PeripheralStretchEdgeInsetUv -Value $PeripheralStretchEdgeInsetUv
}
if (-not [double]::IsNaN($PeripheralStretchMaxInsetUv)) {
    Set-Prop -Name $properties.PeripheralStretchMaxInsetUv -Value $PeripheralStretchMaxInsetUv
}
if (-not [double]::IsNaN($PeripheralStretchCurve)) {
    Set-Prop -Name $properties.PeripheralStretchCurve -Value $PeripheralStretchCurve
}
if (-not [double]::IsNaN($PeripheralStretchInnerBlendUv)) {
    Set-Prop -Name $properties.PeripheralStretchInnerBlendUv -Value $PeripheralStretchInnerBlendUv
}
if (-not [double]::IsNaN($PeripheralStretchBlendCurve)) {
    Set-Prop -Name $properties.PeripheralStretchBlendCurve -Value $PeripheralStretchBlendCurve
}
if ($PeripheralStretchBlendMode) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.PeripheralStretchBlendMode, $PeripheralStretchBlendMode)
}
if ($PeripheralStretchCornerMode) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.PeripheralStretchCornerMode, $PeripheralStretchCornerMode)
}
if ($PeripheralStretchDebug) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.PeripheralStretchDebug, $PeripheralStretchDebug)
}
if (-not [double]::IsNaN($ProjectionDepthMeters)) {
    Set-Prop -Name $properties.ProjectionDepthMeters -Value $ProjectionDepthMeters
}
if (-not [double]::IsNaN($ProjectionAreaDiagnostic)) {
    Set-Prop -Name $properties.ProjectionAreaDiagnostic -Value $ProjectionAreaDiagnostic
}
if (-not [double]::IsNaN($ProjectionAreaLeftUv)) {
    Set-Prop -Name $properties.ProjectionAreaLeftOffsetXUv -Value (-$ProjectionAreaLeftUv)
}
if (-not [double]::IsNaN($ProjectionAreaRightUv)) {
    Set-Prop -Name $properties.ProjectionAreaRightOffsetXUv -Value (-$ProjectionAreaRightUv)
}
if (-not [double]::IsNaN($ProjectionAreaVerticalUv)) {
    Set-Prop -Name $properties.ProjectionAreaOffsetYUv -Value $ProjectionAreaVerticalUv
}
if (-not [double]::IsNaN($ProjectionAreaScaleX)) {
    Set-Prop -Name $properties.ProjectionAreaScaleX -Value $ProjectionAreaScaleX
}
if (-not [double]::IsNaN($ProjectionAreaScaleY)) {
    Set-Prop -Name $properties.ProjectionAreaScaleY -Value $ProjectionAreaScaleY
}
if (-not [double]::IsNaN($ProjectionTargetOffsetXUv)) {
    Set-Prop -Name $properties.ProjectionTargetOffsetXUv -Value $ProjectionTargetOffsetXUv
}
if (-not [double]::IsNaN($ProjectionTargetOffsetYUv)) {
    Set-Prop -Name $properties.ProjectionTargetOffsetYUv -Value $ProjectionTargetOffsetYUv
}
if (-not [double]::IsNaN($ProjectionTargetScale)) {
    Set-Prop -Name $properties.ProjectionTargetScale -Value $ProjectionTargetScale
}
if ($ProjectionTargetJoystickControls) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionTargetJoystickControls, $ProjectionTargetJoystickControls)
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionTargetJoystickControlsLegacy, $ProjectionTargetJoystickControls)
}
if (-not [double]::IsNaN($ProjectionAreaRadiusXUv)) {
    Set-Prop -Name $properties.ProjectionAreaRadiusXUv -Value $ProjectionAreaRadiusXUv
}
if (-not [double]::IsNaN($ProjectionAreaRadiusYUv)) {
    Set-Prop -Name $properties.ProjectionAreaRadiusYUv -Value $ProjectionAreaRadiusYUv
}
if (-not [double]::IsNaN($ProjectionAreaCornerRadiusUv)) {
    Set-Prop -Name $properties.ProjectionAreaCornerRadiusUv -Value $ProjectionAreaCornerRadiusUv
}
if (-not [double]::IsNaN($ProjectionAreaKeystoneX)) {
    Set-Prop -Name $properties.ProjectionAreaKeystoneX -Value $ProjectionAreaKeystoneX
}
if (-not [double]::IsNaN($ProjectionAreaBowX)) {
    Set-Prop -Name $properties.ProjectionAreaBowX -Value $ProjectionAreaBowX
}
if (-not [double]::IsNaN($ProjectionAlphaScale)) {
    Set-Prop -Name $properties.ProjectionAlphaScale -Value $ProjectionAlphaScale
}
if (-not [double]::IsNaN($ProjectionAlphaBias)) {
    Set-Prop -Name $properties.ProjectionAlphaBias -Value $ProjectionAlphaBias
}
if ($UseResolvedProjectionRuntime) {
    Invoke-Adb -Arguments @(
        "shell",
        "setprop",
        $properties.ResolvedProjectionRuntime,
        $UseResolvedProjectionRuntime
    )
}

$readback = foreach ($property in $properties.GetEnumerator()) {
    $value = (Invoke-Adb -Arguments @("shell", "getprop", $property.Value)) -join ""
    [pscustomobject]@{
        key = $property.Key
        property = $property.Value
        value = $value.Trim()
    }
}

$readback | ConvertTo-Json -Depth 3
