param([Parameter(Mandatory=$true)][string]$ProjectRoot)
$ErrorActionPreference='Stop'
$root=[IO.Path]::GetFullPath($ProjectRoot)
$identity=Get-Content -LiteralPath (Join-Path $root 'packaging/store/identity.json') -Raw | ConvertFrom-Json
if($identity.name -ne 'AppteriskSolutionMSDNBHD.AppteriskMessenger' -or $identity.publisher -ne 'CN=836DCC0B-274C-4946-AD67-90F4ED2B574D'){throw 'Unexpected Store package identity'}
$package=Get-Content -LiteralPath (Join-Path $root 'package.json') -Raw | ConvertFrom-Json
$tauri=Get-Content -LiteralPath (Join-Path $root 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
if($package.version -ne $tauri.version -or $package.version -notmatch '^\d+\.\d+\.\d+$'){throw 'Store package requires consistent stable numeric version metadata'}
$version=$package.version+'.0'
$exe=Join-Path $root 'src-tauri/target/store/release/appterisk-messenger.exe'
if(!(Test-Path -LiteralPath $exe)){throw 'Build the Store feature executable first with npm run build:store'}
$sdkRoot=Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
$sdk=Get-ChildItem -LiteralPath $sdkRoot -Directory | Where-Object {$_.Name -match '^10\.\d+\.\d+\.\d+$' -and (Test-Path -LiteralPath (Join-Path $_.FullName 'x64/makeappx.exe'))} | Sort-Object {[version]$_.Name} -Descending | Select-Object -First 1
if(!$sdk){throw 'Install Windows SDK with MakeAppx before packaging'}
$makeappx=Join-Path $sdk.FullName 'x64/makeappx.exe'
$out=Join-Path $root 'store-out'
New-Item -ItemType Directory -Path $out -Force | Out-Null
# A fresh staging directory avoids stale or accidental private files in the package.
$stage=Join-Path $out ('stage-'+[guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path (Join-Path $stage 'Assets') -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'appterisk-messenger.exe')
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination (Join-Path $stage 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $out 'THIRD_PARTY_NOTICES.txt') -Destination (Join-Path $stage 'THIRD_PARTY_NOTICES.txt')
foreach($name in @('StoreLogo.png','Square150x150Logo.png','Square44x44Logo.png')){Copy-Item -LiteralPath (Join-Path $root ('src-tauri/icons/'+$name)) -Destination (Join-Path $stage ('Assets/'+$name))}
$manifest=@"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
 xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
 xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10"
 xmlns:virtualization="http://schemas.microsoft.com/appx/manifest/virtualization/windows10"
 xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
 IgnorableNamespaces="uap uap10 virtualization rescap">
 <Identity Name="$($identity.name)" Publisher="$($identity.publisher)" Version="$version" ProcessorArchitecture="x64" />
 <Properties>
  <DisplayName>Appterisk Messenger</DisplayName>
  <PublisherDisplayName>$($identity.publisherDisplayName)</PublisherDisplayName>
  <Description>Independent messaging accounts in one desktop application.</Description>
  <Logo>Assets\StoreLogo.png</Logo>
  <virtualization:FileSystemWriteVirtualization><virtualization:ExcludedDirectories><virtualization:ExcludedDirectory>`$(KnownFolder:LocalAppData)\com.appterisk.messenger</virtualization:ExcludedDirectory></virtualization:ExcludedDirectories></virtualization:FileSystemWriteVirtualization>
 </Properties>
 <Resources><Resource Language="en-US"/><Resource Language="zh-CN"/><Resource Language="zh-TW"/></Resources>
 <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.22000.0" MaxVersionTested="10.0.26100.0"/></Dependencies>
 <Applications>
  <Application Id="App" Executable="appterisk-messenger.exe" uap10:RuntimeBehavior="packagedClassicApp" uap10:TrustLevel="mediumIL">
   <uap:VisualElements DisplayName="Appterisk Messenger" Description="Independent messaging accounts in one desktop application." Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square44x44Logo.png" BackgroundColor="transparent" />
   <Extensions><uap:Extension Category="windows.protocol"><uap:Protocol Name="appterisk"><uap:DisplayName>Appterisk Messenger</uap:DisplayName></uap:Protocol></uap:Extension></Extensions>
  </Application>
 </Applications>
 <Capabilities><rescap:Capability Name="runFullTrust"/><rescap:Capability Name="unvirtualizedResources"/></Capabilities>
</Package>
"@
[IO.File]::WriteAllText((Join-Path $stage 'AppxManifest.xml'),$manifest,(New-Object Text.UTF8Encoding($false)))
$artifact=Join-Path $out ("Appterisk-Messenger_"+$version+'_x64.msix')
if(Test-Path -LiteralPath $artifact){throw 'MSIX already exists; archive the previous artifact explicitly before packaging another build'}
& $makeappx pack /d $stage /p $artifact /h SHA256
if($LASTEXITCODE -ne 0){throw 'MakeAppx package/schema validation failed'}
$sha=(Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant()
$record=[ordered]@{version=$version;architecture='x64';store_id=$identity.storeId;identity=$identity.name;publisher=$identity.publisher;sha256=$sha;bytes=(Get-Item -LiteralPath $artifact).Length;signature='Unsigned Store submission; Microsoft signing pending certification';makeappx_validation='passed';installed_acceptance='not tested';profile_authentication_acceptance='not tested';built_at=[DateTime]::UtcNow.ToString('o');git_commit=(& git -C $root rev-parse HEAD);staging_directory=$stage}
[IO.File]::WriteAllText(($artifact+'.json'),($record | ConvertTo-Json -Depth 5))
[IO.File]::WriteAllText((Join-Path $out 'SHA256SUMS.txt'),($sha+'  '+[IO.Path]::GetFileName($artifact)+"`n"))
Write-Output ('MSIX prepared and schema-validated: '+$artifact)
Write-Output 'This unsigned package is for Partner Center upload, not direct installation by friends. Store certification and installed acceptance remain pending.'
