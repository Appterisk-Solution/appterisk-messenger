import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { command, messageFor, type Bounds } from '../services/probe';
export interface Account {id:string;name:string;platform:string;profile_key:string;icon:string;display_order:number;notifications_enabled:boolean;created_at:string;updated_at:string;last_active_at:string|null}
export interface Settings {theme:'system'|'dark'|'light';language:'system'|'en'|'ms'|'zh'|'zh-CN'|'zh-TW';default_account:string|null;last_active_account:string|null}
export interface Platform {id:string;name:string;entry_url:string;origin:string;symbol:string;status:string;available:boolean;limitations:string}
export interface Snapshot {macos?:boolean;platforms?:Platform[];accounts:Account[];settings:Settings;legacy:{id:string;name:string}[];version:string;diagnostic:boolean;runtime_version:string|null;synthetic?:boolean;store_distribution?:boolean}
export const useAccounts=defineStore('accounts',()=>{
  const data=ref<Snapshot|null>(null),activeId=ref<string|null>(null),error=ref(''),busy=ref(false),ready=ref(false);
  const active=computed(()=>data.value?.accounts.find(a=>a.id===activeId.value));
  async function run(action:()=>Promise<void>){if(busy.value)return false;busy.value=true;error.value='';try{await action();return true;}catch(e){error.value=messageFor(e);return false;}finally{busy.value=false;}}
  async function refresh(){data.value=await command<Snapshot>('account_snapshot');ready.value=true;}
  async function mutate(name:string,args:Record<string,unknown>){data.value=await command<Snapshot>(name,args);}
  async function select(id:string,bounds:Bounds){await command('account_switch',{id,bounds});activeId.value=id;if(data.value)data.value.settings.last_active_account=id;}
  function startupId(){const s=data.value;return s?.settings.default_account??s?.settings.last_active_account??s?.accounts[0]?.id??null;}
  function reordered(from:string,to:string){const ids=data.value?.accounts.map(a=>a.id)??[];const a=ids.indexOf(from),b=ids.indexOf(to);if(a<0||b<0)return ids;ids.splice(a,1);ids.splice(b,0,from);return ids;}
  return {data,activeId,error,busy,ready,active,run,refresh,mutate,select,startupId,reordered};
});
