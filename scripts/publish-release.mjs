import {readFileSync,readdirSync} from 'node:fs';import {join} from 'node:path';import {Buffer} from 'node:buffer';import {createHash} from 'node:crypto';import {URL} from 'node:url';import {validateRelease} from './release-manifest.mjs';const {fetch}=globalThis;
const repo='Appterisk-Solution/appterisk-messenger',token=process.env.GITHUB_TOKEN;
if(!token)throw new Error('Publishing requires an authorized GitHub Actions token');
const [mode,dir,channel]=process.argv.slice(2);if(!['draft','publish'].includes(mode)||!['stable','beta'].includes(channel))throw new Error('Invalid release operation');
const manifest=JSON.parse(readFileSync(join(dir,'update.json'),'utf8')),tag=`v${manifest.version}`;validateRelease(manifest.version,channel,manifest.platforms['windows-x86_64-nsis'].signature,manifest.platforms['windows-x86_64-nsis'].url);
async function api(path,method='GET',body){const response=await fetch(`https://api.github.com/repos/${repo}/${path}`,{method,headers:{Authorization:`Bearer ${token}`,Accept:'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28',...(body?{'Content-Type':'application/json'}:{})},body:body?JSON.stringify(body):undefined});if(response.status===404&&method==='GET')return null;if(!response.ok)throw new Error(`GitHub operation failed: ${method} ${response.status}`);return response.status===204?null:response.json();}
let release=(await api('releases?per_page=100')).find(r=>r.tag_name===tag)??await api(`releases/tags/${tag}`);
if(mode==='draft'){
 release=release??await api('releases','POST',{tag_name:tag,name:`Appterisk Messenger ${manifest.version}${channel==='beta'?' - experimental prerelease':''}`,draft:true,prerelease:channel==='beta',body:manifest.notes});
 for(const name of readdirSync(dir).filter(n=>/\.(exe|msi|sig|json|txt|md)$/.test(n))){const existing=release.assets.find(a=>a.name===name);if(existing){const expected='sha256:'+createHash('sha256').update(readFileSync(join(dir,name))).digest('hex');if(existing.digest!==expected)throw new Error('Existing immutable release asset differs; refusing to overwrite it');continue;}if(!release.draft)throw new Error('A published release is missing an expected immutable asset');const url=new URL(release.upload_url.split('{')[0]);if(url.protocol!=='https:'||url.hostname!=='uploads.github.com')throw new Error('Invalid GitHub upload endpoint');url.searchParams.set('name',name);const uploaded=await fetch(url,{method:'POST',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/octet-stream'},body:readFileSync(join(dir,name))});if(!uploaded.ok)throw new Error(`Asset upload failed: ${uploaded.status}`);}
 console.log('Signed artifacts uploaded to a draft release; no channel manifest changed');
}else{
 if(!release)throw new Error('Expected an approved release');
 if(release.prerelease!==(channel==='beta'))throw new Error('Release channel mismatch');
 if(release.draft)release=await api(`releases/${release.id}`,'PATCH',{draft:false,prerelease:channel==='beta'});
 if(release.published_at)manifest.pub_date=release.published_at;
 // Only update the feed after the immutable released NSIS bytes are publicly downloadable.
 const platform=manifest.platforms['windows-x86_64-nsis'],response=await fetch(platform.url);if(!response.ok||!response.url.startsWith('https:'))throw new Error('Published installer is not accessible over HTTPS; manifest unchanged');
 const remote=Buffer.from(await response.arrayBuffer()),localName=decodeURIComponent(new URL(platform.url).pathname.split('/').at(-1));const digest=b=>createHash('sha256').update(b).digest('hex');if(digest(remote)!==digest(readFileSync(join(dir,localName))))throw new Error('Published artifact checksum mismatch; manifest unchanged');
 const branch='update-channels';if(!await api(`git/ref/heads/${branch}`)){const base=await api('git/ref/heads/main');await api('git/refs','POST',{ref:`refs/heads/${branch}`,sha:base.object.sha});}
 for(const path of [`${channel}/update.json`,...(channel==='stable'?['update.json']:[])]){const previous=await api(`contents/${path}?ref=${branch}`);await api(`contents/${path}`,'PUT',{message:`Publish ${channel} updater manifest ${manifest.version}`,branch,content:Buffer.from(JSON.stringify(manifest,null,2)+'\n').toString('base64'),...(previous?.sha?{sha:previous.sha}:{})});}
 console.log(`Published ${channel}/update.json after validating the public signed artifact`);
}
