import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';

if (process.platform !== 'darwin') throw new Error('This fixture requires macOS 14+.');
const run = randomUUID();
const executable = resolve('src-tauri/target/debug/appterisk-messenger');
const root = join(homedir(), 'Library/Application Support/com.appterisk.messenger/diagnostic-runs', run);
for (let phase = 0; phase <= 3; phase++) {
  const result = spawnSync(executable, ['--macos-isolation-diagnostic', String(phase), ...(process.env.APPTERISK_MACOS_PROVIDER_SMOKE === '1' ? ['--macos-provider-smoke'] : []), run], { timeout: 120_000, encoding: 'utf8' });
  if (result.error || result.status !== 0) {
    let detail = result.error?.message ?? result.stderr;
    try { detail = JSON.parse(readFileSync(join(root, `phase-${phase}.json`), 'utf8')).error ?? detail; } catch { /* No report if startup failed. */ }
    throw new Error(`Native fixture phase ${phase} failed: ${detail}`);
  }
  const report = JSON.parse(readFileSync(join(root, `phase-${phase}.json`), 'utf8'));
  if (!report.pass || !report.accountManager || Object.keys(report.stores ?? {}).length !== 3) throw new Error(`Storage checks failed: ${JSON.stringify(report)}`);
  console.log(`PASS: three independent stores, phase ${phase}${phase === 0 ? ' (seed)' : ' (full restart)'}`);
}
console.log(`Synthetic evidence: ${root}`);
