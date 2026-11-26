import {spawnSync} from 'node:child_process';
import {join,resolve} from 'node:path';
import {readFileSync} from 'node:fs';
const root=resolve(import.meta.dirname,'..');
if(process.platform!=='win32'||process.arch!=='x64')throw new Error('Store packaging currently requires Windows x64');
const identity=JSON.parse(readFileSync(join(root,'packaging/store/identity.json'),'utf8'));
if(identity.storeId!=='9NRZ58RTHSGZ'||identity.architecture!=='x64')throw new Error('Unexpected Store identity');
const env={...process.env,CARGO_TARGET_DIR:join(root,'src-tauri/target/store')};
// Store builds never need the production Tauri key, Windows PFX or signing passwords.
for(const key of Object.keys(env))if(/^(TAURI_SIGNING_|WINDOWS_CERTIFICATE_)/.test(key))delete env[key];
const build=spawnSync(process.execPath,[join(root,'scripts/tauri.mjs'),'build','--no-bundle','--features','store-distribution','--config',join(root,'src-tauri/tauri.store.conf.json')],{cwd:root,env,stdio:'inherit'});
if(build.status!==0)throw new Error('Store application build failed');
const binary=join(env.CARGO_TARGET_DIR,'release/appterisk-messenger.exe');
const info=spawnSync(binary,['--distribution-info'],{encoding:'utf8',windowsHide:true});
if(info.status!==0)throw new Error('Cannot verify Store executable distribution policy');
const distribution=JSON.parse(info.stdout),pkg=JSON.parse(readFileSync(join(root,'package.json'),'utf8'));
if(distribution.store_distribution!==true||distribution.version!==pkg.version)throw new Error('Refusing to package a mismatched or non-Store executable');
const notices=spawnSync(process.execPath,[join(root,'scripts/store-notices.mjs')],{cwd:root,env,stdio:'inherit'});
if(notices.status!==0)throw new Error('Dependency notice collection failed');
const script=join(root,'scripts/package-store.ps1');
// Execute the checked-in script without changing machine or user execution policy.
const pack=spawnSync('powershell.exe',['-NoProfile','-Command','& ([scriptblock]::Create([IO.File]::ReadAllText($env:APPTERISK_STORE_SCRIPT))) -ProjectRoot $env:APPTERISK_STORE_ROOT'],{cwd:root,env:{...env,APPTERISK_STORE_SCRIPT:script,APPTERISK_STORE_ROOT:root},stdio:'inherit'});
if(pack.status!==0)throw new Error('MSIX packaging failed');
const verify=spawnSync('powershell.exe',['-NoProfile','-Command','& ([scriptblock]::Create([IO.File]::ReadAllText($env:APPTERISK_STORE_SCRIPT))) -ProjectRoot $env:APPTERISK_STORE_ROOT'],{cwd:root,env:{...env,APPTERISK_STORE_SCRIPT:join(root,'scripts/verify-store.ps1'),APPTERISK_STORE_ROOT:root},stdio:'inherit'});
if(verify.status!==0)throw new Error('MSIX content verification failed');
