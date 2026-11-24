import {computed,ref} from 'vue';
import {defineStore} from 'pinia';
import {command} from '../services/probe';
export interface DesktopConfig {notifications:boolean;dnd:boolean;sound:boolean;hide_previews:boolean;launch_on_startup:boolean;start_minimized:boolean;minimize_to_tray:boolean;close_to_tray:boolean;download_directory:string|null;ask_before_saving:boolean;lock_timeout_seconds:number}
export interface Download {id:string;account_id:string;name:string;received:number;total:number|null;state:string;saved_path:string|null}
export interface Permission {account_id:string;origin:string;kind:string;decision:string}
export interface Prompt {id:string;account_id:string;origin:string;kind:string}
export interface DesktopSnapshot {config:DesktopConfig;locked:boolean;lock_enabled:boolean;unread:Record<string,{count:number|null;unread:boolean}>;downloads:Download[];prompts:Prompt[];permissions:Permission[];pending_account:string|null;native_notifications:boolean}
export const useDesktop=defineStore('desktop',()=>{
  const data=ref<DesktopSnapshot|null>(null),ready=ref(false);
  const locked=computed(()=>data.value?.locked??false);
  async function refresh(){data.value=await command<DesktopSnapshot>('desktop_snapshot');ready.value=true;}
  function receive(value:DesktopSnapshot){data.value=value;ready.value=true;}
  async function preference<K extends keyof DesktopConfig>(key:K,value:DesktopConfig[K]){if(!data.value)return;await command('desktop_save',{config:{...data.value.config,[key]:value}});await refresh();}
  return{data,ready,locked,refresh,receive,preference};
});
