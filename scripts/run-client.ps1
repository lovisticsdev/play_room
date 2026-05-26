[CmdletBinding()]
param(
    [string]$Name = "player",
    [string]$HostName = "127.0.0.1",
    [int]$Port = 7878,
    [string]$ReconnectToken,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ExtraArgs = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
Push-Location $RepoRoot
try {
    $ClientArgs = @("--name", $Name, "--host", $HostName, "--port", [string]$Port)
    if ($ReconnectToken) {
        $ClientArgs += @("--reconnect-token", $ReconnectToken)
    }

    $CargoArgs = @("run", "-p", "play-room-client", "--") + $ClientArgs + $ExtraArgs
    & cargo @CargoArgs
}
finally {
    Pop-Location
}
