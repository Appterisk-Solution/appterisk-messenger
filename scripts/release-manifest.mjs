import {readFileSync,writeFileSync,mkdirSync,readdirSync,statSync} from 'node:fs';
import {resolve,join,basename} from 'node:path';import {createHash} from 'node:crypto';import {spawnSync} from 'node:child_process';import {pathToFileURL,URL} from 'node:url';import {Buffer} from 'node:buffer';import semver from 'semver';
export function validateRelease(version,channel,signature,url){
 if(!semver.valid(version)||!['stable','beta'].includes(channel)||(channel==='stable'&&semver.prerelease(version)))throw new Error('Invalid release version or channel');
 const u=new URL(url);if(u.protocol!=='https:'||u.hostname!=='github.com'||u.username||u.password||u.search||u.hash||!u.pathname.startsWith(`/Appterisk-Solution/appterisk-messenger/releases/download/v${version}/`)||!u.pathname.endsWith('x64-setup.exe'))throw new Error('Untrusted installer URL');
 if(!signature?.trim()||!Buffer.from(signature.trim(),'base64').toString('utf8').includes(`version:${version}`))throw new Error('Missing signed version');
}
export function generate(directory,version,channel,notes,out){
 const names=readdirSync(directory);const installer=names.find(n=>n.endsWith('_x64-setup.exe'));if(!installer)throw new Error('Actual NSIS installer is required');
 const url=`https://github.com/Appterisk-Solution/appterisk-messenger/releases/download/v${version}/${encodeURIComponent(installer)}`;
 const signature=readFileSync(join(directory,installer+'.sig'),'utf8').trim();validateRelease(version,channel,signature,url);
 const root=resolve(import.meta.dirname,'..');
 const verified=spawnSync(process.execPath,[join(root,'scripts','cargo.mjs'),'run','--quiet','--manifest-path',join(root,'release-tools','Cargo.toml'),'--',join(root,'src-tauri','update-public-key.txt'),join(directory,installer+'.sig'),join(directory,installer),version],{encoding:'utf8'});
 if(verified.status!==0)throw new Error('Actual artifact signature verification failed');
 const manifest={version,notes:readFileSync(notes,'utf8'),pub_date:new Date().toISOString(),platforms:{'windows-x86_64-nsis':{signature,url}}};
 mkdirSync(resolve(out,'..'),{recursive:true});writeFileSync(out,JSON.stringify(manifest,null,2)+'\n');
 const records=names.filter(n=>/\.(exe|msi|sig)$/.test(n)).map(name=>({name,bytes:statSync(join(directory,name)).size,sha256:createHash('sha256').update(readFileSync(join(directory,name))).digest('hex')}));
 writeFileSync(join(directory,'SHA256SUMS.txt'),records.map(r=>`${r.sha256}  ${r.name}`).join('\n')+'\n');
 writeFileSync(join(directory,'artifacts.json'),JSON.stringify({version,channel,architecture:'x86_64',built_at:new Date().toISOString(),artifacts:records},null,2)+'\n');return manifest;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){const [directory,version,channel,notes,out]=process.argv.slice(2);if(!out)throw new Error('Expected artifact directory, version, channel, notes file, output JSON');generate(directory,version,channel,notes,out);console.log(`Verified ${basename(out)} generated from actual signed NSIS bytes`);}
