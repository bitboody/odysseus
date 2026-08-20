<#
.SYNOPSIS
    Build (or fetch) the Odysseus Tauri launcher inside the VM.

.DESCRIPTION
    1. Installs Chocolatey, Git, Node.js, MSVC build tools, NSIS.
    2. Installs Rust (stable-x86_64-pc-windows-msvc).
    3. Ensures WebView2 Runtime is present.
    4. Installs cargo-tauri CLI.
    5. Clones the Odysseus fork (tauri branch) from GitHub.
    6. Builds the release binary WITH the NSIS installer.
    7. Copies the portable .exe and the installer to C:\OdysseusBuild.
    8. Places a shortcut on the OdysseusUser Desktop.

    The build happens entirely inside the VM; nothing touches your host.

.NOTES
    Run as Administrator. Requires ~12 GB free disk and internet access.
#>

$ErrorActionPreference = "Stop"
$ProgressPreference    = "SilentlyContinue"

# -----------------------------------------------------------------
# Config
# -----------------------------------------------------------------
$RepoUrl      = "https://github.com/bitboody/odysseus.git"
$RepoBranch   = "tauri"
$CloneDir     = "C:\odysseus-build"
$DeployDir    = "C:\OdysseusBuild"
$TestUser     = "OdysseusUser"

Write-Host "[*] Starting build provisioning..." -ForegroundColor Cyan

# -----------------------------------------------------------------
# 1. Chocolatey
# -----------------------------------------------------------------
if (-not (Get-Command choco -ErrorAction SilentlyContinue)) {
    Write-Host "[*] Installing Chocolatey..."
    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol = 'Tls12'
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
    $env:PATH += ";$env:ALLUSERSPROFILE\chocolatey\bin"
} else {
    Write-Host "[*] Chocolatey already present."
}

# -----------------------------------------------------------------
# 2. Core build tools
# -----------------------------------------------------------------
$pkgs = @("git", "nodejs", "nsis", "vcredist140")
foreach ($pkg in $pkgs) {
    choco install $pkg -y --no-progress --limit-output
}

# MSVC tool-chain required by Rust on Windows
choco install visualstudio2022buildtools -y --no-progress --limit-output
choco install visualstudio2022-workload-vctools -y --no-progress --limit-output

# Refresh environment so new binaries are on PATH
refreshenv
$env:PATH += ";$env:ProgramFiles\Git\cmd;$env:ProgramFiles\nodejs"

# -----------------------------------------------------------------
# 3. Rustup + stable MSVC target
# -----------------------------------------------------------------
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "[*] Installing Rust..."
    $rustup = "C:\Windows\Temp\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustup -UseBasicParsing
    Start-Process -FilePath $rustup -ArgumentList "-y --default-toolchain stable-x86_64-pc-windows-msvc" -Wait
    Remove-Item $rustup -Force
} else {
    Write-Host "[*] Rust already installed."
}
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
rustup target add x86_64-pc-windows-msvc | Out-Null

# -----------------------------------------------------------------
# 4. WebView2 Evergreen Runtime
# -----------------------------------------------------------------
$wv2Reg = Get-ChildItem "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
if (-not $wv2Reg) {
    Write-Host "[*] Installing WebView2 Evergreen Runtime..."
    $wv2 = "C:\Windows\Temp\MicrosoftEdgeWebview2Setup.exe"
    Invoke-WebRequest -Uri "https://go.microsoft.com/fwlink/p/?LinkId=2124703" -OutFile $wv2 -UseBasicParsing
    Start-Process -FilePath $wv2 -ArgumentList "/silent /install" -Wait
    Remove-Item $wv2 -Force
} else {
    Write-Host "[*] WebView2 already present."
}

# -----------------------------------------------------------------
# 5. Tauri CLI
# -----------------------------------------------------------------
Write-Host "[*] Installing cargo-tauri CLI..."
cargo install tauri-cli --force | Out-Null
Write-Host "[+] Tauri CLI ready."

# -----------------------------------------------------------------
# 6. Clone source
# -----------------------------------------------------------------
if (Test-Path $CloneDir) {
    Remove-Item -Recurse -Force $CloneDir
}
Write-Host "[*] Cloning '$RepoBranch' branch from '$RepoUrl'..."
& git clone --single-branch --branch $RepoBranch --depth 1 $RepoUrl $CloneDir
if ($LASTEXITCODE -ne 0) { throw "Git clone failed." }

# -----------------------------------------------------------------
# 7. Placeholder dist/ so tauri build succeeds
# -----------------------------------------------------------------
$distDir = "$CloneDir\dist"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Force -Path $distDir | Out-Null
    '<!DOCTYPE html><html><head><title>Odysseus</title></head><body></body></html>' | Out-File -FilePath "$distDir\index.html" -Encoding utf8 -NoNewline
}

# -----------------------------------------------------------------
# 8. Build release binary + NSIS installer
# -----------------------------------------------------------------
Push-Location "$CloneDir\src-tauri"
try {
    Write-Host "[*] Building Odysseus Tauri launcher (release) ..." -ForegroundColor Cyan
    cargo tauri build --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "cargo tauri build failed with code $LASTEXITCODE" }
} finally {
    Pop-Location
}

# -----------------------------------------------------------------
# 9. Stage artefacts
# -----------------------------------------------------------------
$releaseDir = "$CloneDir\src-tauri\target\x86_64-pc-windows-msvc\release"
New-Item -ItemType Directory -Force -Path $DeployDir | Out-Null

# Portable .exe
$portable = "$releaseDir\odysseus.exe"
Copy-Item -Path $portable -Destination "$DeployDir\odysseus.exe" -Force
Write-Host "[+] Portable binary: $DeployDir\odysseus.exe"

# NSIS installer
$nsisDir = "$releaseDir\bundle\nsis"
if (Test-Path $nsisDir) {
    Get-ChildItem -Path $nsisDir -Filter "*.exe" | ForEach-Object {
        Copy-Item $_.FullName -Destination "$DeployDir\$($_.Name)" -Force
        Write-Host "[+] Installer: $DeployDir\$($_.Name)"
    }
}

# Checksums
Get-ChildItem -Path $DeployDir | Get-FileHash -Algorithm SHA256 |
    Out-File -FilePath "$DeployDir\checksums.sha256"

# -----------------------------------------------------------------
# 10. Place shortcut on the test user's desktop
# -----------------------------------------------------------------
$desktop = "C:\Users\$TestUser\Desktop"
New-Item -ItemType Directory -Force -Path $desktop | Out-Null

# Copy the portable .exe directly to the desktop so the user can just double-click it
Copy-Item -Path "$DeployDir\odysseus.exe" -Destination "$desktop\odysseus.exe" -Force

# Also create a .lnk for convenience
$Wsh = New-Object -ComObject WScript.Shell
$lnk = $Wsh.CreateShortcut("$desktop\Odysseus.lnk")
$lnk.TargetPath = "$DeployDir\odysseus.exe"
$lnk.WorkingDirectory = $DeployDir
$lnk.IconLocation = "$DeployDir\odysseus.exe,0"
$lnk.Save()

# Grant the user read/execute access to the deploy directory
$acl = Get-Acl $DeployDir
$rule = New-Object System.Security.AccessControl.FileSystemAccessRule($TestUser, "ReadAndExecute", "ContainerInherit,ObjectInherit", "None", "Allow")
$acl.SetAccessRule($rule)
Set-Acl $DeployDir $acl

Write-Host "`n[+] Build provisioning complete." -ForegroundColor Green
Write-Host "    Executable : $DeployDir\odysseus.exe"
Write-Host "    SHA-256    : $( (Get-FileHash "$DeployDir\odysseus.exe" -Algorithm SHA256).Hash )"
Write-Host ""
Write-Host "    To use it, log in to the VM as '$TestUser' and double-click"
Write-Host "    the Odysseus icon on the Desktop."
