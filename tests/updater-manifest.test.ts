import {describe,it,expect} from 'vitest';
// @ts-expect-error The release generator is a Node ESM build script.
import {validateRelease} from '../scripts/release-manifest.mjs';
describe('release manifest security',()=>{
 const sig=Buffer.from('trusted comment: version:0.5.1').toString('base64');const url='https://github.com/Appterisk-Solution/appterisk-messenger/releases/download/v0.5.1/App_x64-setup.exe';
 it('rejects invalid versions, prereleases on stable and missing signatures',()=>{expect(()=>validateRelease('0.5.1','stable',sig,url)).not.toThrow();for(const v of ['x','0.5','0.6.0-beta.1'])expect(()=>validateRelease(v,'stable',sig,url)).toThrow();expect(()=>validateRelease('0.5.1','stable','',url)).toThrow();});
 it('rejects alternate hosts, repositories, schemes, installer types and version substitution',()=>{for(const u of [url.replace('https:','http:'),url.replace('github.com','evil.test'),url.replace('appterisk-messenger','other'),url.replace('.exe','.msi'),url.replace('v0.5.1','v99.0.0')])expect(()=>validateRelease('0.5.1','stable',sig,u)).toThrow();expect(()=>validateRelease('99.0.0','beta',sig,url.replaceAll('0.5.1','99.0.0'))).toThrow();});
});
