param(
    [string]$Serial = "",
    [double]$Strength = [double]::NaN,
    [double]$GlobalUv = [double]::NaN,
    [double]$LeftUv = [double]::NaN,
    [double]$RightUv = [double]::NaN,
    [double]$VerticalUv = [double]::NaN,
    [double]$SymmetricUv = [double]::NaN,
    [double]$ContentScale = [double]::NaN,
    [double]$ProjectionBorderStrength = [double]::NaN,
    [double]$ProjectionAreaDiagnostic = [double]::NaN,
    [double]$ProjectionAreaLeftUv = [double]::NaN,
    [double]$ProjectionAreaRightUv = [double]::NaN,
    [double]$ProjectionAreaVerticalUv = [double]::NaN,
    [double]$ProjectionAreaScaleX = [double]::NaN,
    [double]$ProjectionAreaScaleY = [double]::NaN,
    [double]$ProjectionAreaKeystoneX = [double]::NaN,
    [double]$ProjectionAreaBowX = [double]::NaN,
    [switch]$Reset
)

$ErrorActionPreference = "Stop"

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
    ProjectionBorderStrength = "debug.rustyxr.makepad.projection.border.strength"
    ProjectionAreaDiagnostic = "debug.rustyxr.makepad.projection.area.diagnostic"
    ProjectionAreaLeftUv = "debug.rustyxr.makepad.projection.area.offset.left.uv"
    ProjectionAreaRightUv = "debug.rustyxr.makepad.projection.area.offset.right.uv"
    ProjectionAreaVerticalUv = "debug.rustyxr.makepad.projection.area.offset.vertical.uv"
    ProjectionAreaScaleX = "debug.rustyxr.makepad.projection.area.scale.x"
    ProjectionAreaScaleY = "debug.rustyxr.makepad.projection.area.scale.y"
    ProjectionAreaKeystoneX = "debug.rustyxr.makepad.projection.area.keystone.x"
    ProjectionAreaBowX = "debug.rustyxr.makepad.projection.area.bow.x"
}

if ($Reset) {
    $Strength = 0.0
    $GlobalUv = 0.0
    $LeftUv = 0.0
    $RightUv = 0.0
    $VerticalUv = 0.0
    $ContentScale = 1.60
    $ProjectionBorderStrength = 1.0
    $ProjectionAreaDiagnostic = 0.0
    $ProjectionAreaLeftUv = 0.0
    $ProjectionAreaRightUv = 0.0
    $ProjectionAreaVerticalUv = 0.0
    $ProjectionAreaScaleX = 1.0
    $ProjectionAreaScaleY = 1.0
    $ProjectionAreaKeystoneX = 0.0
    $ProjectionAreaBowX = 0.0
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
Assert-Range -Name "ProjectionBorderStrength" -Value $ProjectionBorderStrength -Min 0.0 -Max 1.0
Assert-Range -Name "ProjectionAreaDiagnostic" -Value $ProjectionAreaDiagnostic -Min 0.0 -Max 2.0
Assert-Range -Name "ProjectionAreaLeftUv" -Value $ProjectionAreaLeftUv -Min -0.5 -Max 0.5
Assert-Range -Name "ProjectionAreaRightUv" -Value $ProjectionAreaRightUv -Min -0.5 -Max 0.5
Assert-Range -Name "ProjectionAreaVerticalUv" -Value $ProjectionAreaVerticalUv -Min -0.5 -Max 0.5
Assert-Range -Name "ProjectionAreaScaleX" -Value $ProjectionAreaScaleX -Min 0.5 -Max 1.5
Assert-Range -Name "ProjectionAreaScaleY" -Value $ProjectionAreaScaleY -Min 0.5 -Max 1.5
Assert-Range -Name "ProjectionAreaKeystoneX" -Value $ProjectionAreaKeystoneX -Min -0.45 -Max 0.45
Assert-Range -Name "ProjectionAreaBowX" -Value $ProjectionAreaBowX -Min -0.25 -Max 0.25

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
if (-not [double]::IsNaN($ProjectionBorderStrength)) {
    Set-Prop -Name $properties.ProjectionBorderStrength -Value $ProjectionBorderStrength
}
if (-not [double]::IsNaN($ProjectionAreaDiagnostic)) {
    Set-Prop -Name $properties.ProjectionAreaDiagnostic -Value $ProjectionAreaDiagnostic
}
if (-not [double]::IsNaN($ProjectionAreaLeftUv)) {
    Set-Prop -Name $properties.ProjectionAreaLeftUv -Value $ProjectionAreaLeftUv
}
if (-not [double]::IsNaN($ProjectionAreaRightUv)) {
    Set-Prop -Name $properties.ProjectionAreaRightUv -Value $ProjectionAreaRightUv
}
if (-not [double]::IsNaN($ProjectionAreaVerticalUv)) {
    Set-Prop -Name $properties.ProjectionAreaVerticalUv -Value $ProjectionAreaVerticalUv
}
if (-not [double]::IsNaN($ProjectionAreaScaleX)) {
    Set-Prop -Name $properties.ProjectionAreaScaleX -Value $ProjectionAreaScaleX
}
if (-not [double]::IsNaN($ProjectionAreaScaleY)) {
    Set-Prop -Name $properties.ProjectionAreaScaleY -Value $ProjectionAreaScaleY
}
if (-not [double]::IsNaN($ProjectionAreaKeystoneX)) {
    Set-Prop -Name $properties.ProjectionAreaKeystoneX -Value $ProjectionAreaKeystoneX
}
if (-not [double]::IsNaN($ProjectionAreaBowX)) {
    Set-Prop -Name $properties.ProjectionAreaBowX -Value $ProjectionAreaBowX
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
