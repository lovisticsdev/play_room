[CmdletBinding()]
param(
    [string]$HostName = "127.0.0.1",
    [int]$Port = 5173,
    [switch]$Install,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ExtraArgs = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$WebRoot = Join-Path $RepoRoot "web"

Push-Location $WebRoot
try {
    if ($Install -or -not (Test-Path -LiteralPath "node_modules")) {
        & npm install
    }

    $NpmArgs = @("run", "dev", "--", "--host", $HostName, "--port", [string]$Port) + $ExtraArgs
    & npm @NpmArgs
}
finally {
    Pop-Location
}
