param([switch]$Authorized, [int]$Port = 37078, [string]$State = "$env:LOCALAPPDATA\CampusMesh\relay.json")
$ErrorActionPreference = 'Stop'
if (-not $Authorized) { throw 'Only run on your own or explicitly authorized Windows device. Pass -Authorized after obtaining permission. School whiteboards are not a supported deployment target.' }
Set-Location (Join-Path $PSScriptRoot '..')
cargo build --release --locked --bin mesh-node
if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
# No firewall, installation, service registration or privilege changes are made.
# Manually add authorized phone IP:port via the documented peer JSON command.
& '.\target\release\mesh-node.exe' $State 'Authorized relay' $Port
