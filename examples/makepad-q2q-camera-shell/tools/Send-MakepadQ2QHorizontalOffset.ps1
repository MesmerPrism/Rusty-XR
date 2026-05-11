param(
    [string]$Serial = "",
    [double]$Strength = [double]::NaN,
    [double]$GlobalUv = [double]::NaN,
    [double]$LeftUv = [double]::NaN,
    [double]$RightUv = [double]::NaN,
    [double]$SymmetricUv = [double]::NaN,
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
}

if ($Reset) {
    $Strength = 0.425
    $GlobalUv = 0.0
    $LeftUv = 0.0
    $RightUv = 0.0
}

if (-not [double]::IsNaN($SymmetricUv)) {
    $LeftUv = $SymmetricUv
    $RightUv = -$SymmetricUv
}

Assert-Range -Name "Strength" -Value $Strength -Min -4.0 -Max 4.0
Assert-Range -Name "GlobalUv" -Value $GlobalUv -Min -0.5 -Max 0.5
Assert-Range -Name "LeftUv" -Value $LeftUv -Min -0.5 -Max 0.5
Assert-Range -Name "RightUv" -Value $RightUv -Min -0.5 -Max 0.5
Assert-Range -Name "SymmetricUv" -Value $SymmetricUv -Min -0.5 -Max 0.5

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

$readback = foreach ($property in $properties.GetEnumerator()) {
    $value = (Invoke-Adb -Arguments @("shell", "getprop", $property.Value)) -join ""
    [pscustomobject]@{
        key = $property.Key
        property = $property.Value
        value = $value.Trim()
    }
}

$readback | ConvertTo-Json -Depth 3
