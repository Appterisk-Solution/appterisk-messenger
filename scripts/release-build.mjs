import {writeFileSync,mkdtempSync} from 'node:fs';import {tmpdir} from 'node:os';import {join} from 'node:path';import {spawnSync} from 'node:child_process';import {Buffer} from 'node:buffer';
if(!process.env.TAURI_SIGNING_PRIVATE_KEY)throw new Error('Production signing key is mandatory');
const args=['scripts/tauri.mjs','build'];
if(process.env.WINDOWS_CERTIFICATE_BASE64){
 const directory=mkdtempSync(join(tmpdir(),'appterisk-certificate-'));const certificate=join(directory,'publisher.pfx');writeFileSync(certificate,Buffer.from(process.env.WINDOWS_CERTIFICATE_BASE64,'base64'));
 const imported=spawnSync('powershell.exe',['-NoProfile','-Command',String.raw`$ErrorActionPreference='Stop'; $secret=ConvertTo-SecureString $env:WINDOWS_CERTIFICATE_PASSWORD -AsPlainText -Force; $cert=Import-PfxCertificate -FilePath $env:APPTERISK_PFX -CertStoreLocation Cert:\CurrentUser\My -Password $secret; $cert.Thumbprint; Remove-Item -LiteralPath $env:APPTERISK_PFX`],{encoding:'utf8',env:{...process.env,APPTERISK_PFX:certificate}});
 if(imported.status!==0)throw new Error('Publisher certificate import failed; output suppressed');const thumbprint=imported.stdout.trim();if(!/^[A-F0-9]{40}$/i.test(thumbprint))throw new Error('A single code signing certificate is required');
 const configuration=join(directory,'signing.json');writeFileSync(configuration,JSON.stringify({bundle:{windows:{certificateThumbprint:thumbprint,digestAlgorithm:'sha256',timestampUrl:'http://timestamp.digicert.com'}}}));args.push('--config',configuration);
}
const result=spawnSync(process.execPath,args,{stdio:'inherit'});if(result.status!==0)throw new Error('Release build failed');
if(process.env.WINDOWS_CERTIFICATE_BASE64){const checked=spawnSync('powershell.exe',['-NoProfile','-File','scripts/verify-authenticode.ps1'],{stdio:'inherit'});if(checked.status!==0)throw new Error('Publisher signature or timestamp validation failed');}
