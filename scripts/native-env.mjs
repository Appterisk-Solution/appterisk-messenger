import { execFileSync, execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

// npm can be launched from Git Bash, PowerShell, or an editor with stale PATH.
// Configure only this child process; do not replace the user's shell settings.
export const nativeEnv = { ...process.env };
const env = nativeEnv;
export let toolsReady = process.platform !== 'win32';
if (process.platform === 'win32') {
  const pathKey = Object.keys(env).find(key => key.toLowerCase() === 'path') ?? 'Path';
  env[pathKey] = `${join(env.CARGO_HOME ?? join(homedir(), '.cargo'), 'bin')};${env[pathKey] ?? ''}`;
  const vswhere = join(env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)', 'Microsoft Visual Studio', 'Installer', 'vswhere.exe');
  if (existsSync(vswhere)) {
    const installation = execFileSync(vswhere, ['-latest', '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath'], { encoding: 'utf8', env }).trim();
    const devcmd = join(installation, 'Common7', 'Tools', 'VsDevCmd.bat');
    if (installation && existsSync(devcmd)) {
      const output = execSync(`"${devcmd}" -no_logo -arch=x64 -host_arch=x64 >nul && set`, { encoding: 'utf8', env, shell: env.ComSpec ?? 'cmd.exe' });
      for (const line of output.split(/\r?\n/)) {
        const separator = line.indexOf('=');
        if (separator <= 0) continue;
        const key = line.slice(0, separator);
        const previous = Object.keys(env).find(existing => existing.toLowerCase() === key.toLowerCase());
        if (previous) delete env[previous];
        env[key] = line.slice(separator + 1);
      }
      toolsReady = true;
    }
  }
}
