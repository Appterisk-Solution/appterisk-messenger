param([Parameter(Mandatory=$true)][string]$ProjectRoot)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$root=[IO.Path]::GetFullPath($ProjectRoot)
$version=(Get-Content -LiteralPath (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version+'.0'
$artifact=Join-Path $root ('store-out/Appterisk-Messenger_'+$version+'_x64.msix')
$record=Get-Content -LiteralPath ($artifact+'.json') -Raw | ConvertFrom-Json
if((Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant() -ne $record.sha256){throw 'Store package checksum mismatch'}
$allowed=@('AppxManifest.xml','appterisk-messenger.exe','LICENSE.txt','THIRD_PARTY_NOTICES.txt','Assets/StoreLogo.png','Assets/Square150x150Logo.png','Assets/Square44x44Logo.png','AppxBlockMap.xml','[Content_Types].xml')
$zip=[IO.Compression.ZipFile]::OpenRead($artifact)
try {
  $names=@($zip.Entries | ForEach-Object {$_.FullName.Replace('\','/')})
  foreach($name in $names){if($name -notin $allowed){throw ('Unexpected package content: '+$name)}}
  foreach($name in $allowed){if($name -notin $names){throw ('Required package file missing: '+$name)}}
  $stream=$zip.GetEntry('AppxManifest.xml').Open()
  $reader=New-Object IO.StreamReader($stream)
  try{[xml]$xml=$reader.ReadToEnd()}finally{$reader.Dispose()}
  if($xml.Package.Identity.Name -ne 'AppteriskSolutionMSDNBHD.AppteriskMessenger' -or $xml.Package.Identity.Publisher -ne 'CN=836DCC0B-274C-4946-AD67-90F4ED2B574D' -or $xml.Package.Identity.Version -ne $version -or $xml.Package.Identity.ProcessorArchitecture -ne 'x64'){throw 'Package manifest identity mismatch'}
  if($xml.Package.Applications.Application.Executable -ne 'appterisk-messenger.exe'){throw 'Unexpected application entry point'}
  $namespace=New-Object Xml.XmlNamespaceManager($xml.NameTable)
  $namespace.AddNamespace('v','http://schemas.microsoft.com/appx/manifest/virtualization/windows10')
  $exclude=$xml.SelectSingleNode('//v:ExcludedDirectory',$namespace)
  if(!$exclude -or $exclude.InnerText -cne '$(KnownFolder:LocalAppData)\com.appterisk.messenger'){throw 'Profile virtualization exclusion is missing or too broad'}
  $hash=[Security.Cryptography.SHA256]::Create()
  $stream=$zip.GetEntry('appterisk-messenger.exe').Open()
  try{$payload=([BitConverter]::ToString($hash.ComputeHash($stream))).Replace('-','').ToLowerInvariant()}finally{$stream.Dispose();$hash.Dispose()}
  if($payload -ne (Get-FileHash -LiteralPath (Join-Path $root 'src-tauri/target/store/release/appterisk-messenger.exe') -Algorithm SHA256).Hash.ToLowerInvariant()){throw 'Packaged application differs from the Store build'}
} finally {$zip.Dispose()}
Write-Output 'Store package checksum, exact identity, file allowlist, profile exclusion and executable payload verified.'
Write-Output 'No package installation, signing or Microsoft certification was performed by this check.'
