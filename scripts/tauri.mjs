import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { nativeEnv, toolsReady } from './native-env.mjs';
const args = process.argv.slice(2);
if (['dev', 'build'].includes(args[0]) && !args.includes('--help') && !toolsReady) {
  console.error('Microsoft C++ Build Tools are missing. Install Visual Studio Build Tools with "Desktop development with C++" and Windows SDK, then rerun this command. Cargo is configured automatically; Git Bash is supported.');
  process.exit(1);
}
const require = createRequire(import.meta.url);
const child = spawn(process.execPath, [require.resolve('@tauri-apps/cli/tauri.js'), ...args], { env: nativeEnv, stdio: 'inherit' });
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
