// Development-only codemod. Runtime localization uses explicit t() template calls.
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {parse as parseSfc} from '@vue/compiler-sfc';
import {parse} from '@vue/compiler-dom';
const root=resolve(import.meta.dirname,'..'),path=resolve(root,'src/App.vue');
const dictionary=readFileSync(resolve(root,'src/i18n.ts'),'utf8');
const keys=new Set([...dictionary.matchAll(/\['((?:[^'\\]|\\.)*)',/gm)].map(m=>m[1].replace(/\\'/g,"'")));
const source=readFileSync(path,'utf8'),template=parseSfc(source).descriptor.template;
const base=source.indexOf(template.content),edits=[];
function literal(text){return JSON.stringify(text);}
function visit(node){
  if(node.type===2){const text=node.content.trim();if(keys.has(text)&&text){const leading=node.content.match(/^\s*/)[0],trailing=node.content.match(/\s*$/)[0];edits.push({start:base+node.loc.start.offset,end:base+node.loc.end.offset,text:`${leading}{{ t(${literal(text)}) }}${trailing}`});}}
  if(node.type===5){const expr=node.content;const transformed=expr.content.replace(/'((?:[^'\\]|\\.)*)'/g,(whole,text,offset)=>keys.has(text)&&!expr.content.slice(0,offset).trimEnd().endsWith('t(')?`t(${whole})`:whole);if(transformed!==expr.content)edits.push({start:base+expr.loc.start.offset,end:base+expr.loc.end.offset,text:transformed});}
  for(const prop of node.props??[]){if(prop.type===6&&['aria-label','title','placeholder'].includes(prop.name)&&prop.value&&keys.has(prop.value.content)){edits.push({start:base+prop.loc.start.offset,end:base+prop.loc.end.offset,text:`:${prop.name}='t(${literal(prop.value.content).replace(/'/g,'&#39;')})'`});}}
  for(const child of node.children??[])visit(child);
}
visit(parse(template.content));let result=source;for(const edit of edits.sort((a,b)=>b.start-a.start)){result=result.slice(0,edit.start)+edit.text+result.slice(edit.end);}writeFileSync(path,result);
console.log(`Localized ${edits.length} template nodes/attributes.`);
