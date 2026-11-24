import {spawnSync} from 'node:child_process';
import {join,resolve} from 'node:path';
if(process.platform!=='win32')throw new Error('Installer upgrade verification requires Windows.');
const root=resolve(import.meta.dirname,'..');
const stage=process.argv.find(a=>a.startsWith('--stage='))?.slice(8);
if(stage && !['updater'].includes(stage))throw new Error('Unknown validation stage');
// Execute our local checked-in test code as a command, without changing execution policy.
const result=spawnSync('powershell.exe',['-NoProfile','-Command','& ([scriptblock]::Create([IO.File]::ReadAllText($env:APPTERISK_UPGRADE_SCRIPT))) -ProjectRoot $env:APPTERISK_UPGRADE_ROOT -ReportDirectory $env:APPTERISK_UPGRADE_REPORT -Refresh:($env:APPTERISK_UPGRADE_REFRESH -eq "1")'],{stdio:'inherit',env:{...process.env,APPTERISK_UPGRADE_SCRIPT:join(root,'scripts','verify-upgrade-0.5.0.ps1'),APPTERISK_UPGRADE_ROOT:root,APPTERISK_UPGRADE_REPORT:join(root,'docs','release-evidence-0.5.0',...(stage?[stage]:[])),APPTERISK_UPGRADE_REFRESH:process.argv.includes('--refresh')?'1':'0'}});
if(result.error)throw result.error;
process.exitCode=result.status??1;
