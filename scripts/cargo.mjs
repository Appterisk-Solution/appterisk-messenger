import { spawn } from 'node:child_process';
import { nativeEnv } from './native-env.mjs';
const child = spawn('cargo', process.argv.slice(2), { env: nativeEnv, stdio: 'inherit' });
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
