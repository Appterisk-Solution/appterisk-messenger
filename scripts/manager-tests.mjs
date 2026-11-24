import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {setTimeout,clearTimeout} from 'node:timers';
if(process.platform!=='win32')throw new Error('Native manager checks require Windows/WebView2.');
const run=randomUUID(),root=join(process.env.LOCALAPPDATA,'com.appterisk.messenger');
const exe=join(import.meta.dirname,'..','src-tauri','target','debug','appterisk-messenger.exe');
for(const [cycle,stage] of ['seed','verify','verify','verify','remove'].entries()){
  let pid;
  await new Promise((resolve,reject)=>{
    const child=spawn(exe,['--manager-diagnostic',stage,run],{stdio:'inherit'});pid=child.pid;
    const timer=setTimeout(()=>{child.unref();reject(new Error('Manager checks timed out. The app was left open; close normally.'));},180000);
    child.once('error',e=>{clearTimeout(timer);reject(e);});child.once('exit',code=>{clearTimeout(timer);if(code===0)resolve();else reject(new Error(`Manager ${stage} exited ${code}`));});
  });
  const reportPath=join(root,'diagnostic-runs',run,`manager-${stage}.json`),report=JSON.parse(readFileSync(reportPath,'utf8'));
  if(!report.passed)throw new Error(JSON.stringify(report));
  const log=readFileSync(join(root,'logs','persistence.jsonl'),'utf8').trim().split(/\r?\n/).map(line=>JSON.parse(line)).filter(e=>e.pid===pid);
  for(const id of report.account_ids){if(!log.some(e=>e.event==='browser_process_exited'&&e.fields.profile_id===id&&e.fields.exit_kind===0))throw new Error(`Missing normal BrowserProcessExited acknowledgment for ${id}`);}
  if(!log.some(e=>e.event==='shutdown_complete'&&e.fields.browser_processes_exited))throw new Error('Centralized graceful shutdown was not acknowledged.');
  writeFileSync(join(root,'diagnostic-runs',run,`manager-cycle-${cycle}.json`),JSON.stringify({...report,normal_browser_exit:true,centralized_shutdown:true},null,2));
  console.log(`Manager ${stage} ${cycle}: passed with three independent native profiles and normal shutdown.`);
}
console.log(`Manager reports: ${join(root,'diagnostic-runs',run)}`);
