import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import {join} from 'node:path';
import {readFileSync} from 'node:fs';
import {setTimeout,clearTimeout} from 'node:timers';
if(process.platform!=='win32')throw new Error('Native link checks require Windows.');
const run=randomUUID(),exe=join(import.meta.dirname,'../src-tauri/target/debug/appterisk-messenger.exe');
await new Promise((resolve,reject)=>{
  const child=spawn(exe,['tg://resolve?domain=telegram','--manager-diagnostic','chat-links',run],{stdio:'inherit'});
  const timer=setTimeout(()=>{child.unref();reject(new Error('Link check timed out; close the diagnostic app normally.'));},120000);
  child.once('error',e=>{clearTimeout(timer);reject(e);});
  child.once('exit',code=>{clearTimeout(timer);if(code===0)resolve();else reject(new Error(`Native link check failed (${code}).`));});
});
const report=JSON.parse(readFileSync(join(process.env.LOCALAPPDATA,'com.appterisk.messenger/diagnostic-runs',run,'manager-chat-links.json'),'utf8'));
for(const flag of ['passed','cold_start','warm_activation','official_client_navigation','account_mismatch_rejected','stale_request_rejected','lock_enforced','remote_ipc_denied'])if(report[flag]!==true)throw new Error(`Missing link evidence: ${flag}`);
console.log(`Native link checks passed in disposable profiles: ${run}`);
