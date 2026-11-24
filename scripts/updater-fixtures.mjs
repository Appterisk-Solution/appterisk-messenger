import {mkdtempSync,writeFileSync,mkdirSync,copyFileSync} from 'node:fs';
import {tmpdir} from 'node:os';import {join,resolve} from 'node:path';import {spawnSync} from 'node:child_process';
const root=resolve(import.meta.dirname,'..'),out=join(root,'tests','updater-fixtures');mkdirSync(out,{recursive:true});
const temp=mkdtempSync(join(tmpdir(),'appterisk-disposable-updater-'));
function run(args){const r=spawnSync(process.execPath,[join(root,'scripts','tauri.mjs'),...args],{encoding:'utf8'});if(r.status!==0)throw new Error('Disposable fixture signing failed; output suppressed');}
for(const id of ['public','wrong-public']){const key=join(temp,id+'.key');run(['signer','generate','--ci','-p','disposable-fixture-only','-w',key]);copyFileSync(key+'.pub',join(out,id+'.txt'));if(id==='public'){writeFileSync(join(out,'payload.txt'),'Public updater signature fixture. No executable or credentials.\n');run(['signer','sign','-f',key,'-p','disposable-fixture-only','--app-version','0.5.1',join(out,'payload.txt')]);}}
console.log('Public signed fixture created; disposable private keys remain outside the repository.');
