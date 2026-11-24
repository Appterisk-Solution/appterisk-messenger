param([string]$ProjectRoot = (Join-Path $PSScriptRoot '..'), [string]$ReportDirectory = (Join-Path $ProjectRoot 'docs/release-evidence'), [switch]$Refresh)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath($ProjectRoot)
$dataRoot = Join-Path $env:LOCALAPPDATA 'com.appterisk.messenger'
$installRoot = Join-Path $env:LOCALAPPDATA 'Appterisk Messenger'
$baselineInstaller = Join-Path $projectRoot 'src-tauri/target/release/bundle/nsis/Appterisk Messenger_0.1.2_x64-setup.exe'
$upgradeInstaller = Join-Path $projectRoot 'src-tauri/target/release/bundle/nsis/Appterisk Messenger_0.2.0_x64-setup.exe'
$installedExecutable = Join-Path $installRoot 'appterisk-messenger.exe'
if (Get-Process -Name appterisk-messenger -ErrorAction SilentlyContinue) { throw 'Close all Appterisk applications normally before the upgrade test.' }
if (!(Test-Path -LiteralPath $baselineInstaller) -or !(Test-Path -LiteralPath $upgradeInstaller)) { throw 'Both release installers must exist before the upgrade test.' }
if (Test-Path -LiteralPath $installedExecutable) {
    $existingVersion = (Get-Item -LiteralPath $installedExecutable).VersionInfo.ProductVersion
    if ($existingVersion -notlike '0.1.2*' -and !($Refresh -and $existingVersion -like '0.2.0*')) { throw 'Refusing to downgrade an existing installation for this test.' }
}
function Get-ProfileMetadata {
    $metadata = @{}
    foreach ($namespace in @('compatibility-profiles','accounts')) {
        $directory = Join-Path $dataRoot $namespace
        if (Test-Path -LiteralPath $directory) {
            Get-ChildItem -LiteralPath $directory -Recurse -File | ForEach-Object {
                # Only filesystem metadata. Never open browser databases or their contents.
                $metadata[$_.FullName] = "$($_.Length):$($_.LastWriteTimeUtc.Ticks)"
            }
        }
    }
    return ,$metadata
}
function Assert-ProfileMetadata($before, $after) {
    if ($before.Count -ne $after.Count) { throw 'Profile file count changed during installation.' }
    foreach ($key in $before.Keys) { if ($after[$key] -ne $before[$key]) { throw 'Profile file metadata changed during installation.' } }
}
$before = Get-ProfileMetadata
if (!(Test-Path -LiteralPath $installedExecutable)) {
    $baseline = Start-Process -FilePath $baselineInstaller -ArgumentList '/S' -WindowStyle Hidden -PassThru -Wait
    if ($baseline.ExitCode -ne 0) { throw "Baseline installer exited $($baseline.ExitCode)." }
}
$baselineVersion = (Get-Item -LiteralPath $installedExecutable).VersionInfo.ProductVersion
if ($baselineVersion -notlike '0.1.2*' -and !($Refresh -and $baselineVersion -like '0.2.0*')) { throw 'Expected installed baseline version 0.1.2.' }
Assert-ProfileMetadata $before (Get-ProfileMetadata)
$upgrade = Start-Process -FilePath $upgradeInstaller -ArgumentList '/S','/UPDATE' -WindowStyle Hidden -PassThru -Wait
if ($upgrade.ExitCode -ne 0) { throw "Upgrade installer exited $($upgrade.ExitCode)." }
$upgradedVersion = (Get-Item -LiteralPath $installedExecutable).VersionInfo.ProductVersion
if ($upgradedVersion -notlike '0.2.0*') { throw 'Expected installed upgraded version 0.2.0.' }
Assert-ProfileMetadata $before (Get-ProfileMetadata)
$installedHash = (Get-FileHash -LiteralPath $installedExecutable -Algorithm SHA256).Hash
$releaseHash = (Get-FileHash -LiteralPath (Join-Path $projectRoot 'src-tauri/target/release/appterisk-messenger.exe') -Algorithm SHA256).Hash
# Tauri temporarily patches UNK to NSS when bundling NSIS, then restores the raw exe.
# Check the entire payload hash after that exact, documented three-byte in-memory patch.
$releaseBytes = [IO.File]::ReadAllBytes((Join-Path $projectRoot 'src-tauri/target/release/appterisk-messenger.exe'))
$releaseAscii = [Text.Encoding]::ASCII.GetString($releaseBytes)
$marker = '__TAURI_BUNDLE_TYPE_VAR_UNK'
$markerIndex = $releaseAscii.IndexOf($marker, [StringComparison]::Ordinal)
if ($markerIndex -lt 0 -or $releaseAscii.IndexOf($marker, $markerIndex + 1, [StringComparison]::Ordinal) -ge 0) { throw 'Expected one Tauri bundle-type marker in the raw executable.' }
$nsisMarker = [Text.Encoding]::ASCII.GetBytes('__TAURI_BUNDLE_TYPE_VAR_NSS')
[Array]::Copy($nsisMarker, 0, $releaseBytes, $markerIndex, $nsisMarker.Length)
$hasher = [Security.Cryptography.SHA256]::Create()
try { $expectedPayloadHash = [BitConverter]::ToString($hasher.ComputeHash($releaseBytes)).Replace('-', '') } finally { $hasher.Dispose() }
if ($installedHash -ne $expectedPayloadHash) { throw 'Installed executable does not match the expected final NSIS payload.' }
$report = [ordered]@{
    date = '2026-11-24'; installer = 'NSIS x64'; baseline_version = $baselineVersion; upgraded_version = $upgradedVersion
    upgrade_exit_code = $upgrade.ExitCode; profile_file_count = $before.Count; profile_file_metadata_unchanged = $true
    browser_database_contents_read = $false; authentication_data_copied = $false
    real_authentication_after_upgrade = 'Requires user validation'; msi_upgrade_tested = $false
    installed_executable_matches_expected_nsis_payload = $true; installed_executable_sha256 = $installedHash
    raw_release_executable_sha256 = $releaseHash; expected_bundle_marker = '__TAURI_BUNDLE_TYPE_VAR_NSS'
}
New-Item -ItemType Directory -Path $ReportDirectory -Force | Out-Null
$reportName = if ($Refresh) { 'nsis-final-package.json' } else { 'nsis-upgrade.json' }
$report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $ReportDirectory $reportName) -Encoding utf8
$report | ConvertTo-Json
