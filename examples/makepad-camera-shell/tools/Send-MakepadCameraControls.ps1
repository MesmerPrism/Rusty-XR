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
    [ValidateSet("", "off", "scale")]
    [string]$ProjectionTargetBreathControls = "",
    [string]$ProjectionTargetBreathStream = "",
    [double]$ProjectionTargetBreathMinScale = [double]::NaN,
    [double]$ProjectionTargetBreathMaxScale = [double]::NaN,
    [double]$ProjectionTargetBreathSmoothingAlpha = [double]::NaN,
    [ValidateSet("", "true", "false")]
    [string]$ProjectionTargetBreathInvert = "",
    [double]$ProjectionTargetBreathMinQuality = [double]::NaN,
    [ValidateSet("", "true", "false")]
    [string]$ManifoldBreathFeedbackEnabled = "",
    [string]$ManifoldBreathFeedbackStream = "",
    [string]$ManifoldBreathFeedbackReceiver = "",
    [string]$ManifoldBrokerHost = "",
    [int]$ManifoldBrokerPort = 0,
    [int]$ManifoldBreathFeedbackConnectTimeoutMs = 0,
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
    Strength = "debug.rustyquest.makepad.horizontal.alignment.strength"
    GlobalUv = "debug.rustyquest.makepad.horizontal.offset.uv"
    LeftUv = "debug.rustyquest.makepad.horizontal.offset.left.uv"
    RightUv = "debug.rustyquest.makepad.horizontal.offset.right.uv"
    VerticalUv = "debug.rustyquest.makepad.vertical.offset.uv"
    ContentScale = "debug.rustyquest.makepad.content.uv.scale"
    ProjectionBorderOpacity = "debug.rustyquest.makepad.projection.border.opacity"
    ProjectionBorderPolicy = "debug.rustyquest.makepad.projection.border.policy"
    NativePassthroughEnabled = "debug.rustyquest.makepad.native.passthrough.enabled"
    ProcessingLayer = "debug.rustyquest.makepad.processing.layer"
    ProjectionSampleMode = "debug.rustyquest.makepad.projection.sample.mode"
    BlurRadiusPx = "debug.rustyquest.makepad.camera.blur.radius.px"
    PeripheralStretchMode = "debug.rustyquest.makepad.peripheral.stretch.mode"
    PeripheralStretchCoreScale = "debug.rustyquest.makepad.peripheral.stretch.core.scale"
    PeripheralStretchEdgeInsetUv = "debug.rustyquest.makepad.peripheral.stretch.edge.inset.uv"
    PeripheralStretchMaxInsetUv = "debug.rustyquest.makepad.peripheral.stretch.max.inset.uv"
    PeripheralStretchCurve = "debug.rustyquest.makepad.peripheral.stretch.curve"
    PeripheralStretchInnerBlendUv = "debug.rustyquest.makepad.peripheral.stretch.inner.blend.uv"
    PeripheralStretchBlendCurve = "debug.rustyquest.makepad.peripheral.stretch.blend.curve"
    PeripheralStretchBlendMode = "debug.rustyquest.makepad.peripheral.stretch.blend.mode"
    PeripheralStretchCornerMode = "debug.rustyquest.makepad.peripheral.stretch.corner.mode"
    PeripheralStretchDebug = "debug.rustyquest.makepad.peripheral.stretch.debug"
    ProjectionDepthMeters = "debug.rustyquest.makepad.projection.depth.meters"
    ProjectionAreaDiagnostic = "debug.rustyquest.makepad.projection.area.diagnostic"
    ProjectionAreaLeftOffsetXUv = "debug.rustyquest.makepad.projection.area.offset.left.uv"
    ProjectionAreaRightOffsetXUv = "debug.rustyquest.makepad.projection.area.offset.right.uv"
    ProjectionAreaOffsetYUv = "debug.rustyquest.makepad.projection.area.offset.vertical.uv"
    ProjectionAreaScaleX = "debug.rustyquest.makepad.projection.area.scale.x"
    ProjectionAreaScaleY = "debug.rustyquest.makepad.projection.area.scale.y"
    ProjectionTargetOffsetXUv = "debug.rustyquest.makepad.projection.target.offset.x.uv"
    ProjectionTargetOffsetYUv = "debug.rustyquest.makepad.projection.target.offset.y.uv"
    ProjectionTargetScale = "debug.rustyquest.makepad.projection.target.scale"
    ProjectionTargetJoystickControls = "debug.rustyquest.makepad.projection.target.joystick.controls"
    ProjectionTargetBreathControls = "debug.rustyquest.makepad.projection.target.breath.controls"
    ProjectionTargetBreathStream = "debug.rustyquest.makepad.projection.target.breath.stream"
    ProjectionTargetBreathMinScale = "debug.rustyquest.makepad.projection.target.breath.min.scale"
    ProjectionTargetBreathMaxScale = "debug.rustyquest.makepad.projection.target.breath.max.scale"
    ProjectionTargetBreathSmoothingAlpha = "debug.rustyquest.makepad.projection.target.breath.smoothing.alpha"
    ProjectionTargetBreathInvert = "debug.rustyquest.makepad.projection.target.breath.invert"
    ProjectionTargetBreathMinQuality = "debug.rustyquest.makepad.projection.target.breath.min.quality"
    ManifoldBreathFeedbackEnabled = "debug.rusty.manifold.breath.feedback.enabled"
    ManifoldBreathFeedbackStream = "debug.rusty.manifold.breath.feedback.stream"
    ManifoldBreathFeedbackReceiver = "debug.rusty.manifold.breath.feedback.receiver"
    ManifoldBrokerHost = "debug.rusty.manifold.broker.host"
    ManifoldBrokerPort = "debug.rusty.manifold.broker.port"
    ManifoldBreathFeedbackConnectTimeoutMs = "debug.rusty.manifold.breath.feedback.connect.timeout.ms"
    ProjectionAreaRadiusXUv = "debug.rustyquest.makepad.projection.area.radius.x.uv"
    ProjectionAreaRadiusYUv = "debug.rustyquest.makepad.projection.area.radius.y.uv"
    ProjectionAreaCornerRadiusUv = "debug.rustyquest.makepad.projection.area.corner.radius.uv"
    ProjectionAreaKeystoneX = "debug.rustyquest.makepad.projection.area.keystone.x"
    ProjectionAreaBowX = "debug.rustyquest.makepad.projection.area.bow.x"
    ProjectionAlphaMode = "debug.rustyquest.makepad.projection.alpha.mode"
    ProjectionAlphaScale = "debug.rustyquest.makepad.projection.alpha.scale"
    ProjectionAlphaBias = "debug.rustyquest.makepad.projection.alpha.bias"
    ResolvedProjectionRuntime = "debug.rustyquest.makepad.projection.runtime.resolution.enabled"
}

if ($Reset) {
    $Strength = 0.0
    $GlobalUv = 0.0
    $LeftUv = 0.0
    $RightUv = 0.0
    $VerticalUv = 0.0
    $ContentScale = 1.60
    $ProjectionBorderOpacity = 1.0
    $ProjectionBorderPolicy = "passthrough-underlay"
    $ProcessingLayer = "peripheral-stretch"
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
    $ProjectionDepthMeters = 1.434085
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
    $ProjectionTargetBreathControls = "off"
    $ProjectionTargetBreathStream = "stream.breath.feedback_state"
    $ProjectionTargetBreathMinScale = 1.0
    $ProjectionTargetBreathMaxScale = 1.0
    $ProjectionTargetBreathSmoothingAlpha = 0.30
    $ProjectionTargetBreathInvert = "false"
    $ProjectionTargetBreathMinQuality = 0.0
    $ManifoldBreathFeedbackEnabled = "false"
    $ManifoldBreathFeedbackStream = "stream.breath.feedback_state"
    $ManifoldBreathFeedbackReceiver = "app.makepad_camera_shell.breath_feedback"
    $ManifoldBrokerHost = "127.0.0.1"
    $ManifoldBrokerPort = 8765
    $ManifoldBreathFeedbackConnectTimeoutMs = 250
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
Assert-Range -Name "ProjectionTargetBreathMinScale" -Value $ProjectionTargetBreathMinScale -Min 0.05 -Max 1.50
Assert-Range -Name "ProjectionTargetBreathMaxScale" -Value $ProjectionTargetBreathMaxScale -Min 0.05 -Max 1.50
Assert-Range -Name "ProjectionTargetBreathSmoothingAlpha" -Value $ProjectionTargetBreathSmoothingAlpha -Min 0.0 -Max 1.0
Assert-Range -Name "ProjectionTargetBreathMinQuality" -Value $ProjectionTargetBreathMinQuality -Min 0.0 -Max 1.0
Assert-Range -Name "ProjectionAreaRadiusXUv" -Value $ProjectionAreaRadiusXUv -Min 0.05 -Max 0.5
Assert-Range -Name "ProjectionAreaRadiusYUv" -Value $ProjectionAreaRadiusYUv -Min 0.05 -Max 0.5
Assert-Range -Name "ProjectionAreaCornerRadiusUv" -Value $ProjectionAreaCornerRadiusUv -Min 0.0 -Max 0.5
Assert-Range -Name "ProjectionAreaKeystoneX" -Value $ProjectionAreaKeystoneX -Min -0.45 -Max 0.45
Assert-Range -Name "ProjectionAreaBowX" -Value $ProjectionAreaBowX -Min -0.25 -Max 0.25
Assert-Range -Name "ProjectionAlphaScale" -Value $ProjectionAlphaScale -Min 0.0 -Max 4.0
Assert-Range -Name "ProjectionAlphaBias" -Value $ProjectionAlphaBias -Min -1.0 -Max 1.0
if ($ManifoldBrokerPort -lt 0 -or $ManifoldBrokerPort -gt 65535) {
    throw "ManifoldBrokerPort must be within [0, 65535]; got $ManifoldBrokerPort"
}
if ($ManifoldBreathFeedbackConnectTimeoutMs -lt 0) {
    throw "ManifoldBreathFeedbackConnectTimeoutMs must be non-negative; got $ManifoldBreathFeedbackConnectTimeoutMs"
}

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
}
if ($ProjectionSampleMode) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionSampleMode, $ProjectionSampleMode)
}
if (-not [double]::IsNaN($BlurRadiusPx)) {
    Set-Prop -Name $properties.BlurRadiusPx -Value $BlurRadiusPx
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
}
if ($ProjectionTargetBreathControls) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionTargetBreathControls, $ProjectionTargetBreathControls)
}
if ($ProjectionTargetBreathStream) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionTargetBreathStream, $ProjectionTargetBreathStream)
}
if (-not [double]::IsNaN($ProjectionTargetBreathMinScale)) {
    Set-Prop -Name $properties.ProjectionTargetBreathMinScale -Value $ProjectionTargetBreathMinScale
}
if (-not [double]::IsNaN($ProjectionTargetBreathMaxScale)) {
    Set-Prop -Name $properties.ProjectionTargetBreathMaxScale -Value $ProjectionTargetBreathMaxScale
}
if (-not [double]::IsNaN($ProjectionTargetBreathSmoothingAlpha)) {
    Set-Prop -Name $properties.ProjectionTargetBreathSmoothingAlpha -Value $ProjectionTargetBreathSmoothingAlpha
}
if ($ProjectionTargetBreathInvert) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ProjectionTargetBreathInvert, $ProjectionTargetBreathInvert)
}
if (-not [double]::IsNaN($ProjectionTargetBreathMinQuality)) {
    Set-Prop -Name $properties.ProjectionTargetBreathMinQuality -Value $ProjectionTargetBreathMinQuality
}
if ($ManifoldBreathFeedbackEnabled) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ManifoldBreathFeedbackEnabled, $ManifoldBreathFeedbackEnabled)
}
if ($ManifoldBreathFeedbackStream) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ManifoldBreathFeedbackStream, $ManifoldBreathFeedbackStream)
}
if ($ManifoldBreathFeedbackReceiver) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ManifoldBreathFeedbackReceiver, $ManifoldBreathFeedbackReceiver)
}
if ($ManifoldBrokerHost) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ManifoldBrokerHost, $ManifoldBrokerHost)
}
if ($ManifoldBrokerPort -gt 0) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ManifoldBrokerPort, $ManifoldBrokerPort.ToString([Globalization.CultureInfo]::InvariantCulture))
}
if ($ManifoldBreathFeedbackConnectTimeoutMs -gt 0) {
    Invoke-Adb -Arguments @("shell", "setprop", $properties.ManifoldBreathFeedbackConnectTimeoutMs, $ManifoldBreathFeedbackConnectTimeoutMs.ToString([Globalization.CultureInfo]::InvariantCulture))
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
