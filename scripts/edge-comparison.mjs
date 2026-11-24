// Manual provider comparison only. No cookies, credentials or browser data are copied/read.
import { existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
if (process.platform !== 'win32') throw new Error('This comparison currently targets Windows.');
const candidates = [process.env['ProgramFiles(x86)'], process.env.ProgramFiles]
  .filter(Boolean).map(root=>join(root,'Microsoft','Edge','Application','msedge.exe'));
const edge=candidates.find(path=>existsSync(path));
if (!edge) throw new Error('Microsoft Edge was not found.');
const profile=join(process.env.LOCALAPPDATA,'com.appterisk.messenger','edge-comparison-profile');
mkdirSync(profile,{recursive:true});
const child=spawn(edge,[`--user-data-dir=${profile}`,'--new-window','https://web.whatsapp.com/'],{stdio:'ignore',detached:true});
child.on('error',error=>{console.error(error.message);process.exitCode=1;});
child.unref();
console.log(`Opened normal persistent Edge comparison profile: ${profile}`);
console.log('Link manually, close Edge normally, and rerun this command. No authentication data is copied or inspected.');
