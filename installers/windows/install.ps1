# Windows PowerShell Installation Script for Inactive Host Agent
# Registers host-agent as a SYSTEM-level Windows Service with Firewall rule

#Requires -RunAsAdministrator

Write-Host "=== Inactive Agent — Windows Service Installer ===" -ForegroundColor Cyan

$ServiceName = "InactiveAgent"
$DisplayName = "Inactive Host Service"
$Description = "Inactive - High-performance encrypted remote desktop host service."
$InstallDir = "C:\Program Files\Inactive"
$BinaryPath = "$InstallDir\inactive-agent.exe"

# 1. Create directory
Write-Host "1. Creating installation directory..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# 2. Copy binary
$SourceBin = "$PSScriptRoot\..\..\target\release\host-agent.exe"
if (Test-Path $SourceBin) {
    Write-Host "2. Copying binary..."
    Copy-Item $SourceBin -Destination $BinaryPath -Force
} else {
    Write-Warning "Binary not found at $SourceBin. Please compile first: cargo build --release -p host-agent"
}

# 3. Stop and remove existing service if present
if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
    Write-Host "3. Stopping existing service..."
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    sc.exe delete $ServiceName | Out-Null
    Start-Sleep -Seconds 2
}

# 4. Create Windows Service
Write-Host "4. Creating Windows Service (SYSTEM account)..."
New-Service -Name $ServiceName `
    -BinaryPathName "`"$BinaryPath`" --service" `
    -DisplayName $DisplayName `
    -Description $Description `
    -StartupType Automatic

# 5. Add Windows Firewall Rule for QUIC/UDP and HTTPS/TCP
Write-Host "5. Configuring Windows Firewall..."
New-NetFirewallRule -DisplayName "Inactive Agent Inbound UDP" `
    -Direction Inbound `
    -Action Allow `
    -Protocol UDP `
    -Program $BinaryPath `
    -ErrorAction SilentlyContinue | Out-Null

New-NetFirewallRule -DisplayName "Inactive Agent Inbound TCP" `
    -Direction Inbound `
    -Action Allow `
    -Protocol TCP `
    -Program $BinaryPath `
    -ErrorAction SilentlyContinue | Out-Null

# 6. Start Service
Write-Host "6. Starting service..."
Start-Service -Name $ServiceName

Write-Host "✅ Inactive Service installed and started successfully!" -ForegroundColor Green
Write-Host "   Service Status: " -NoNewline
(Get-Service -Name $ServiceName).Status
