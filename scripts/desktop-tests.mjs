import {spawn} from 'node:child_process';
import {join} from 'node:path';
import {readFileSync} from 'node:fs';
import {randomUUID} from 'node:crypto';
import {setTimeout,clearTimeout} from 'node:timers';
if(process.platform!=='win32')throw new Error('Desktop integration requires Windows/WebView2.');
const run=randomUUID(),path=join(process.env.LOCALAPPDATA,'com.appterisk.messenger','diagnostic-runs',run,'desktop.json');
await new Promise((resolve,reject)=>{const child=spawn(join(import.meta.dirname,'..','src-tauri','target','debug','appterisk-messenger.exe'),['--desktop-diagnostic',run],{stdio:'inherit'});const timer=setTimeout(()=>{child.unref();reject(new Error('Desktop fixture timed out; the app remains open for normal closure.'));},90000);child.once('error',e=>{clearTimeout(timer);reject(e);});child.once('exit',code=>{clearTimeout(timer);if(code===0)resolve();else reject(new Error(`Desktop fixture exited ${code}; see ${path}`));});});
const report=JSON.parse(readFileSync(path,'utf8'));if(!report.passed)throw new Error(JSON.stringify(report));console.log(`Desktop fixture report: ${path}`);
