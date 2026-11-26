import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {join} from 'node:path';
import {setTimeout,clearTimeout} from 'node:timers';
if(process.platform!=='win32')throw new Error('Requires native Windows/WebView2.');
const exe=join(import.meta.dirname,'../src-tauri/target/debug/appterisk-messenger.exe');
for(let profile=0;profile<3;profile++){
  const run=randomUUID();let account;
  for(let restart=0;restart<2;restart++){
    await new Promise((resolve,reject)=>{
      const child=spawn(exe,['--manager-diagnostic','telegram-first-load',run],{stdio:'inherit'});
      const timeout=setTimeout(()=>{child.unref();reject(new Error('Test timed out; close diagnostic app normally.'));},120000);
      child.once('error',error=>{clearTimeout(timeout);reject(error);});
      child.once('exit',code=>{clearTimeout(timeout);if(code===0)resolve();else reject(new Error(`Telegram first-load check failed (${code}).`));});
    });
    const report=JSON.parse(readFileSync(join(process.env.LOCALAPPDATA,'com.appterisk.messenger/diagnostic-runs',run,'manager-telegram-first-load.json'),'utf8'));
    if(!report.passed||!report.telegram_first_load_rendered||report.reload!==false||!report.account_id)throw new Error('Missing first-load evidence.');
    if(account&&account!==report.account_id)throw new Error('Restart did not reuse the account.');
    account=report.account_id;
    console.log(`Telegram profile ${profile+1}, launch ${restart+1}: visible controls without reload; run ${run}`);
  }
}
