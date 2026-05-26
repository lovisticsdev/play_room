[CmdletBinding()]
param(
    [string]$Config = "examples/server.toml",
    [string]$HostName,
    [int]$Port,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ExtraArgs = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
Push-Location $RepoRoot
try {
    $ServerArgs = @("--config", $Config)
    if ($HostName) {
        $ServerArgs += @("--host", $HostName)
    }
    if ($PSBoundParameters.ContainsKey("Port")) {
        $ServerArgs += @("--port", [string]$Port)
    }

    $CargoArgs = @("run", "-p", "play-room-server", "--") + $ServerArgs + $ExtraArgs
    & cargo @CargoArgs
}
finally {
    Pop-Location
}
