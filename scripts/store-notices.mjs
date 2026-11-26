import {readFileSync,readdirSync,writeFileSync,mkdirSync,existsSync} from 'node:fs';
import {join,dirname,resolve} from 'node:path';
import {spawnSync} from 'node:child_process';
const root=resolve(import.meta.dirname,'..'),out=join(root,'store-out');
const cargo=spawnSync(process.execPath,[join(root,'scripts/cargo.mjs'),'metadata','--manifest-path',join(root,'src-tauri/Cargo.toml'),'--format-version','1','--locked','--filter-platform','x86_64-pc-windows-msvc','--features','store-distribution'],{cwd:root,encoding:'utf8',maxBuffer:20*1024*1024});
if(cargo.status!==0)throw new Error('Cannot collect Rust dependency notices');
const metadata=JSON.parse(cargo.stdout),used=new Set(metadata.resolve.nodes.map(n=>n.id));
const entries=[];
function legalFiles(directory){return readdirSync(directory,{withFileTypes:true}).filter(e=>e.isFile()&&/^(licen[cs]e|copying|notice|copyright)([-_.]|$)/i.test(e.name)).map(e=>({name:e.name,text:readFileSync(join(directory,e.name),'utf8')}));}
for(const p of metadata.packages.filter(p=>p.source?.startsWith('registry+')&&used.has(p.id))){entries.push({name:p.name,version:p.version,license:p.license,source:p.repository??p.source,files:legalFiles(dirname(p.manifest_path))});}
const lock=JSON.parse(readFileSync(join(root,'package-lock.json'),'utf8'));
for(const [path,p] of Object.entries(lock.packages).filter(([path,p])=>path&& !p.dev)){
 const directory=join(root,path);if(p.optional&&!existsSync(join(directory,'package.json')))continue;const info=JSON.parse(readFileSync(join(directory,'package.json'),'utf8'));
 entries.push({name:info.name,version:info.version,license:info.license??p.license,source:typeof info.repository==='string'?info.repository:info.repository?.url,files:legalFiles(directory)});
}
const text=['Appterisk Messenger third-party dependency notices','Generated from the installed lockfile dependencies and Cargo metadata. Inclusion does not change third-party licenses. Microsoft WebView2 is a separately installed Microsoft runtime; remote messaging websites are not bundled Appterisk source.','This inventory is not a completed legal/compliance audit. Packages without separate notice files are identified below.',''];
for(const p of entries.sort((a,b)=>`${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`))){text.push(`=== ${p.name} ${p.version} ===`,`Declared license: ${p.license??'Not declared'}`,`Source: ${p.source??'See package registry'}`,...(p.files.length?p.files.flatMap(f=>[`--- ${f.name} ---`,f.text]):['No separate root license/notice file was present; review upstream package terms.']),'');}
mkdirSync(out,{recursive:true});writeFileSync(join(out,'THIRD_PARTY_NOTICES.txt'),text.join('\n'));
writeFileSync(join(out,'dependency-notice-inventory.json'),JSON.stringify(entries.map(({name,version,license,source,files})=>({name,version,license,source,notice_files:files.map(f=>f.name)})),null,2)+'\n');
console.log('Collected dependency license declarations and available license/notice files.');
