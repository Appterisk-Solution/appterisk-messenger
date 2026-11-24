param([ValidateSet('Init','Build')][string]$Action='Build',[string]$ProjectRoot=(Get-Location).Path)
$ErrorActionPreference='Stop'
$root=[IO.Path]::GetFullPath($ProjectRoot)
$vault=Join-Path $env:USERPROFILE '.tauri\appterisk-messenger'
$key=Join-Path $vault 'production.key'
$passwordFile=Join-Path $vault 'password.dpapi'
if($Action -eq 'Init') {
  if(Test-Path -LiteralPath $key){throw 'Production key already exists; refusing to regenerate it.'}
  New-Item -ItemType Directory -Path $vault -Force | Out-Null
  $acl=New-Object Security.AccessControl.DirectorySecurity
  $acl.SetAccessRuleProtection($true,$false)
  $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User
  $rule=New-Object Security.AccessControl.FileSystemAccessRule($sid,'FullControl','ContainerInherit,ObjectInherit','None','Allow')
  $acl.SetOwner($sid);$acl.AddAccessRule($rule);Set-Acl -LiteralPath $vault -AclObject $acl
  $bytes=New-Object byte[] 32
  $rng=[Security.Cryptography.RandomNumberGenerator]::Create();$rng.GetBytes($bytes);$rng.Dispose()
  $password=[Convert]::ToBase64String($bytes)
  $protected=ConvertFrom-SecureString (ConvertTo-SecureString $password -AsPlainText -Force)
  [IO.File]::WriteAllText($passwordFile,$protected)
  $result=& node (Join-Path $root 'scripts/tauri.mjs') signer generate --ci -w $key -p $password 2>&1
  if($LASTEXITCODE -ne 0){throw 'Tauri key generation failed; output suppressed to protect key material.'}
  $public=[IO.File]::ReadAllText(($key+'.pub')).Trim()
  [IO.File]::WriteAllText((Join-Path $root 'src-tauri/update-public-key.txt'),$public+"`n")
  $password=$null;$result=$null
  Write-Output 'Encrypted production updater key created outside the repository. Offline recovery backup still requires the owner.'
} else {
  if(!(Test-Path -LiteralPath $key) -or !(Test-Path -LiteralPath $passwordFile)){throw 'Run signing-key.ps1 Init once as the owner first.'}
  $secure=ConvertTo-SecureString ([IO.File]::ReadAllText($passwordFile))
  $credential=New-Object Management.Automation.PSCredential('updater',$secure)
  $env:TAURI_SIGNING_PRIVATE_KEY=$key
  $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$credential.GetNetworkCredential().Password
  try { & node (Join-Path $root 'scripts/tauri.mjs') build; if($LASTEXITCODE -ne 0){throw 'Signed updater build failed.'} }
  finally { Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue;Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue }
}
