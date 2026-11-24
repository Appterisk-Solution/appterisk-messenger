$ErrorActionPreference='Stop'
$files=@(Get-Item -LiteralPath 'src-tauri/target/release/appterisk-messenger.exe')
$files+=Get-ChildItem -LiteralPath 'src-tauri/target/release/bundle/nsis' -Filter '*_0.5.0_x64-setup.exe'
$files+=Get-ChildItem -LiteralPath 'src-tauri/target/release/bundle/msi' -Filter '*_0.5.0_x64_en-US.msi'
foreach($file in $files){$signature=Get-AuthenticodeSignature -LiteralPath $file.FullName;if($signature.Status -ne 'Valid' -or !$signature.TimeStamperCertificate){throw 'Authenticode chain or trusted timestamp verification failed.'}}
Write-Output 'Windows publisher signatures and timestamps verified.'
