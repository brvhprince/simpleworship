<#
.SYNOPSIS
    Packages SimpleWorship into an MSIX package for Microsoft Store submission.

.DESCRIPTION
    Assembles the required Windows Store package layout (binary, resources, AppxManifest.xml, and Store Assets),
    and executes MakeAppx.exe from the Windows 10/11 SDK.

.PARAMETER IdentityName
    The Package/Identity Name reserved in Microsoft Partner Center.
    (e.g., 'PennyCodes.SimpleWorship' or 'SimpleWorship')

.PARAMETER PublisherId
    The Publisher ID from Microsoft Partner Center (e.g., 'CN=12345678-ABCD-EF01-2345-6789ABCDEF01').

.PARAMETER PublisherDisplayName
    The Publisher Display Name registered in Partner Center (e.g., 'Prince Takyi Akomea').

.PARAMETER Version
    4-part version number, e.g., '0.5.2.0'. Default: derived from apps/desktop/src-tauri/tauri.conf.json or package.json.

.PARAMETER OutputDir
    Directory to place the final .msix package. Default: 'dist-store'.

.PARAMETER SignForLocalTest
    Switch to generate a local self-signed certificate and sign the MSIX for local testing.

.EXAMPLE
    .\scripts\package-msix.ps1 -IdentityName "PennyCodes.SimpleWorship" -PublisherId "CN=12345678-ABCD-EF01-2345-6789ABCDEF01" -PublisherDisplayName "Prince Takyi Akomea"
#>

[CmdletBinding()]
param(
    [string]$IdentityName = "SimpleWorship",

    [string]$PublisherId = "",

    [string]$PublisherDisplayName = "SimpleWorship",

    [string]$Version = "",

    [string]$OutputDir = "dist-store",

    [switch]$SignForLocalTest
)

$ErrorActionPreference = "Stop"

# Root project paths
$RootDir = Resolve-Path "$PSScriptRoot\.."
$TauriDir = Join-Path $RootDir "apps\desktop\src-tauri"
$IconsDir = Join-Path $TauriDir "icons"
$TargetReleaseDir = Join-Path $RootDir "target\release"

$ProductName = "SimpleWorship"
$DefaultExe = "SimpleWorship.exe"
$Description = "Free, open-source worship presentation software."
$ManifestTemplate = Join-Path $TauriDir "windows\AppxManifest.xml.template"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Microsoft Store MSIX Packager - $ProductName" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Extract Version if not specified
if (-not $Version) {
    $TauriConfPath = Join-Path $TauriDir "tauri.conf.json"
    $PkgJsonPath = Join-Path $RootDir "package.json"
    if (Test-Path $TauriConfPath) {
        $TauriConf = Get-Content $TauriConfPath -Raw | ConvertFrom-Json
        $RawVersion = $TauriConf.version
    }
    if (-not $RawVersion -and (Test-Path $PkgJsonPath)) {
        $PkgJson = Get-Content $PkgJsonPath -Raw | ConvertFrom-Json
        $RawVersion = $PkgJson.version
    }
    if (-not $RawVersion) {
        $RawVersion = "0.5.2"
    }
    $Version = $RawVersion
}

# Ensure 4-part version: 0.5.2 -> 0.5.2.0
$VersionParts = $Version.Split(".")
while ($VersionParts.Count -lt 4) {
    $VersionParts += "0"
}
$Version = ($VersionParts[0..3] -join ".")

Write-Host "App Name:         $ProductName"
Write-Host "Version:          $Version"
Write-Host "Identity Name:    $IdentityName"
Write-Host "Publisher:        $PublisherId"
Write-Host "Publisher Name:   $PublisherDisplayName"

# 2. Locate compiled binary
$ExePath = Join-Path $TargetReleaseDir $DefaultExe
if (-not (Test-Path $ExePath)) {
    # Check lower-case variant
    $AltExe = Join-Path $TargetReleaseDir "simpleworship.exe"
    if (Test-Path $AltExe) {
        $ExePath = $AltExe
    } else {
        Write-Warning "Executable not found at $ExePath."
        Write-Host "Attempting build via 'pnpm --filter @simpleworship/desktop tauri build --no-bundle'..." -ForegroundColor Yellow
        Push-Location $RootDir
        try {
            pnpm --filter @simpleworship/desktop tauri build --no-bundle
        } finally {
            Pop-Location
        }
        if (-not (Test-Path $ExePath)) {
            throw "Failed to locate compiled executable at $ExePath after build."
        }
    }
}
$ActualExeName = Split-Path $ExePath -Leaf
Write-Host "Found Executable: $ExePath ($ActualExeName)" -ForegroundColor Green

# 3. Find MakeAppx.exe and SignTool.exe from Windows SDK
function Find-WindowsSdkTool([string]$ToolName) {
    $SdkPaths = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "${env:ProgramFiles}\Windows Kits\10\bin"
    )
    foreach ($base in $SdkPaths) {
        if (Test-Path $base) {
            $Found = Get-ChildItem -Path $base -Recurse -Filter $ToolName -ErrorAction SilentlyContinue |
                     Where-Object { $_.FullName -like "*\x64\*" } |
                     Sort-Object LastWriteTime -Descending |
                     Select-Object -First 1
            if ($Found) {
                return $Found.FullName
            }
        }
    }
    # Fallback to PATH
    $FromPath = Get-Command $ToolName -ErrorAction SilentlyContinue
    if ($FromPath) {
        return $FromPath.Source
    }
    return $null
}

$MakeAppx = Find-WindowsSdkTool "makeappx.exe"
if (-not $MakeAppx) {
    throw "makeappx.exe not found! Please ensure the Windows 10/11 SDK is installed (via Visual Studio Installer or standalone Windows SDK)."
}
Write-Host "Using MakeAppx:   $MakeAppx" -ForegroundColor Green

# 4. Prepare Staging Directory
$StagingDir = Join-Path $RootDir "target\msix-staging"
if (Test-Path $StagingDir) {
    Remove-Item $StagingDir -Recurse -Force
}
New-Item -ItemType Directory -Path $StagingDir -Force | Out-Null

$StagingAssetsDir = Join-Path $StagingDir "Assets"
New-Item -ItemType Directory -Path $StagingAssetsDir -Force | Out-Null

# Copy binary
Copy-Item $ExePath -Destination (Join-Path $StagingDir $ActualExeName) -Force

# Copy any companion DLLs next to the executable if present
Get-ChildItem -Path $TargetReleaseDir -Filter "*.dll" | ForEach-Object {
    Copy-Item $_.FullName -Destination $StagingDir -Force
}

# Copy bundled resource folders (e.g. bibles, remote)
$ExcludeDirs = @("build", "deps", "examples", "incremental", ".fingerprint", "msix-staging")
Get-ChildItem -Path $TargetReleaseDir -Directory | Where-Object { $_.Name -notin $ExcludeDirs } | ForEach-Object {
    Write-Host "Copying bundled resource directory: $($_.Name)" -ForegroundColor Gray
    Copy-Item $_.FullName -Destination (Join-Path $StagingDir $_.Name) -Recurse -Force
}

# Copy Assets / Icons
$AssetFiles = @(
    "Square44x44Logo.png",
    "Square71x71Logo.png",
    "Square150x150Logo.png",
    "Square310x310Logo.png",
    "StoreLogo.png"
)
foreach ($asset in $AssetFiles) {
    $AssetSource = Join-Path $IconsDir $asset
    if (Test-Path $AssetSource) {
        Copy-Item $AssetSource -Destination (Join-Path $StagingAssetsDir $asset) -Force
    } else {
        Write-Warning "Icon asset $asset not found in $IconsDir"
    }
}

# Generate AppxManifest.xml
if (-not (Test-Path $ManifestTemplate)) {
    throw "Manifest template not found at $ManifestTemplate"
}
$ManifestContent = Get-Content $ManifestTemplate -Raw

$ManifestContent = $ManifestContent `
    -replace "\{\{IDENTITY_NAME\}\}", $IdentityName `
    -replace "\{\{PUBLISHER_ID\}\}", $(if ($PublisherId) { $PublisherId } else { "CN=SimpleWorship" }) `
    -replace "\{\{PUBLISHER_DISPLAY_NAME\}\}", $PublisherDisplayName `
    -replace "\{\{PACKAGE_VERSION\}\}", $Version `
    -replace "\{\{PRODUCT_NAME\}\}", $ProductName `
    -replace "\{\{DISPLAY_NAME\}\}", $ProductName `
    -replace "\{\{EXECUTABLE_NAME\}\}", $ActualExeName `
    -replace "\{\{DESCRIPTION\}\}", $Description

$TargetManifest = Join-Path $StagingDir "AppxManifest.xml"
Set-Content -Path $TargetManifest -Value $ManifestContent -Encoding UTF8
Write-Host "Created AppxManifest.xml in staging directory." -ForegroundColor Green

# 5. Create Output Directory
$FullOutputDir = Join-Path $RootDir $OutputDir
if (-not (Test-Path $FullOutputDir)) {
    New-Item -ItemType Directory -Path $FullOutputDir -Force | Out-Null
}

$OutputPackageName = "${ProductName}_${Version}_x64.msix"
$OutputPackagePath = Join-Path $FullOutputDir $OutputPackageName

if (Test-Path $OutputPackagePath) {
    Remove-Item $OutputPackagePath -Force
}

# 6. Execute MakeAppx
Write-Host "Creating MSIX package..." -ForegroundColor Cyan
& $MakeAppx pack /d "$StagingDir" /p "$OutputPackagePath" /o /nv
if ($LASTEXITCODE -ne 0) {
    throw "MakeAppx failed with exit code $LASTEXITCODE"
}

Write-Host "`nSUCCESS! Generated MSIX Package:" -ForegroundColor Green
Write-Host "$OutputPackagePath" -ForegroundColor Yellow

# 7. Optional Local Signing for Sideload Testing
if ($SignForLocalTest) {
    Write-Host "`nSigning package for local testing..." -ForegroundColor Cyan
    $SignTool = Find-WindowsSdkTool "signtool.exe"
    if ($SignTool) {
        $CertSubj = if ($PublisherId) { $PublisherId } else { "CN=SimpleWorship" }
        Write-Host "Creating self-signed dev certificate for $CertSubj..."
        $Cert = New-SelfSignedCertificate `
            -Type Custom `
            -Subject $CertSubj `
            -KeyUsage DigitalSignature `
            -FriendlyName "$ProductName Dev Cert" `
            -CertStoreLocation "Cert:\CurrentUser\My" `
            -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")

        # Sign using SHA256
        & $SignTool sign /fd SHA256 /sha1 $Cert.Thumbprint "$OutputPackagePath"
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Successfully signed package with test certificate ($($Cert.Thumbprint))." -ForegroundColor Green
            Write-Host "NOTE: To install this test package on Windows, trust the cert in 'Trusted People' cert store." -ForegroundColor Gray
        } else {
            Write-Warning "SignTool exited with code $LASTEXITCODE."
        }
    } else {
        Write-Warning "signtool.exe not found. Skipping local signing."
    }
}

# Cleanup staging
Remove-Item $StagingDir -Recurse -Force
Write-Host "`nDone! Ready for Microsoft Partner Center upload.`n" -ForegroundColor Green
