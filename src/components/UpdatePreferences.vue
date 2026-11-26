<script setup lang="ts">
import {onMounted,onUnmounted,ref} from 'vue';
import {listen,type UnlistenFn} from '@tauri-apps/api/event';
import {isTauri} from '@tauri-apps/api/core';
import {command} from '../services/probe';
import {useAccounts} from '../stores/accounts';
import {t} from '../i18n';
interface Snapshot {store_managed?:boolean;preferences:{channel:'stable'|'beta';automatic:boolean};status:string;version:string|null;notes:string;received:number;total:number|null;last_check:number|null;error:string|null;configured:boolean;install_supported:boolean}
const accounts=useAccounts(),data=ref<Snapshot>(),channel=ref<'stable'|'beta'>('stable'),automatic=ref(true),betaConfirmed=ref(false),installConfirmed=ref(false),showNotes=ref(true),working=ref(false),localError=ref(false);
let stop:UnlistenFn|undefined,disposed=false;
async function refresh(){data.value=await command<Snapshot>('update_snapshot');if(data.value){channel.value=data.value.preferences.channel;automatic.value=data.value.preferences.automatic;}}
async function action(name:string,args:Record<string,unknown>={}){working.value=true;localError.value=false;try{await command(name,args);}catch{localError.value=true;}finally{await refresh();working.value=false;}}
onMounted(async()=>{await refresh();if(isTauri()){const off=await listen<Snapshot>('update-state',event=>{data.value=event.payload;});if(disposed)off();else stop=off;}});
onUnmounted(()=>{disposed=true;stop?.();});
function errorMessage(){const error=data.value?.error??'';
 if(error.startsWith('Browser shutdown'))return t('Browser shutdown failed. No update was installed. Close other instances and retry.');
 if(error.startsWith('Finish active'))return t('Finish active downloads and permission prompts before installing.');
 if(error.startsWith('Installer'))return t('The installer could not start. Check disk space and installation permissions, then retry.');
 if(error.includes('signature')||error.includes('manifest'))return t('The update signature or manifest is invalid. Nothing was installed.');
 if(error.includes('cancelled'))return t('Download was cancelled or too large. Nothing was installed.');
 return t('The update could not finish. Check your connection or signing configuration and retry. Your messaging data is unchanged.');
}
function status(value:string){return t(({idle:'Ready to check',checking:'Checking for updates',current:'You are up to date',available:'Update available',downloading:'Downloading update',ready:'Update verified and ready',installing:'Closing browsers and installing',error:'Update operation failed'} as Record<string,string>)[value]??'Update operation failed');}
</script>
<template>
  <section><p class="eyebrow">{{t('APPLICATION UPDATES')}}</p><h3>{{t('Updates')}}</h3>
    <p v-if="data?.store_managed" class="section-description">{{t("Updates are managed by Microsoft Store. Open the Store Library to check for updates.")}}</p>
    <p v-if="data?.store_managed">{{t('Current version')}}: {{accounts.data?.version}}</p>
    <p v-else-if="accounts.data?.macos" class="field-help">Automatic updates are unavailable on macOS. Install a new application bundle manually.</p>
    <template v-else>
    <p class="section-description">{{t('Updates preserve your accounts and require confirmation before installation.')}}</p>
    <dl class="about-details"><dt>{{t('Current version')}}</dt><dd>{{accounts.data?.version}}</dd><dt>WebView2 Evergreen</dt><dd>{{accounts.data?.runtime_version??t('Not available')}}</dd><dt>{{t('Latest available version')}}</dt><dd>{{data?.version??t('Not available')}}</dd><dt>{{t('Last update check')}}</dt><dd>{{data?.last_check?new Date(data.last_check*1000).toLocaleString():t('Never')}}</dd></dl>
    <p v-if="data&&!data.configured&&!accounts.data?.macos" class="field-help">{{t('This build has no configured update signing key. Messaging remains available.')}}</p>
    <p v-if="data&&!data.install_supported" class="field-help">{{t('Automatic installation is currently available for Windows x64 NSIS installations only.')}}</p>
    <label class="field-label" for="update-channel">{{t('Release channel')}}</label><select id="update-channel" v-model="channel" :disabled="working"><option value="stable">{{t('Stable')}}</option><option value="beta">{{t('Beta')}}</option></select>
    <p v-if="channel==='beta'" class="field-help">{{t('Beta builds are experimental. Returning to Stable never installs an older version.')}}</p>
    <label v-if="channel==='beta'" class="checkbox-label"><input v-model="betaConfirmed" type="checkbox"/>{{t('I explicitly opt in to beta updates')}}</label>
    <label class="setting-toggle"><span>{{t('Automatically check on startup and every six hours')}}</span><input v-model="automatic" type="checkbox" :disabled="working"/></label>
    <button class="secondary-button" :disabled="working||(channel==='beta'&&!betaConfirmed)" @click="action('update_preferences',{preferences:{channel,automatic},betaConfirmed})">{{t('Save update preferences')}}</button>
    <p v-if="data" role="status">{{status(data.status)}}</p>
    <progress v-if="data?.status==='downloading'" :value="data.total?data.received:undefined" :max="data.total??1"/>
    <p v-if="data?.status==='downloading'">{{Math.round(data.received/1024)}} KB <span v-if="data.total">/ {{Math.round(data.total/1024)}} KB</span></p>
    <p v-if="data?.error||localError" class="dialog-error" role="alert">{{errorMessage()}}</p>
    <div class="security-actions"><button v-if="data?.status==='error'" class="text-button" :disabled="working" @click="action('update_check')">{{t('Retry')}}</button><button class="secondary-button" :disabled="working||!data?.configured||['checking','downloading','installing'].includes(data?.status??'')" @click="action('update_check')">{{t('Check for Updates')}}</button>
      <button v-if="data?.version&&['available','error'].includes(data.status)" class="primary-button" :disabled="working||!data.install_supported" @click="action('update_download')">{{t('Download Update')}}</button>
      <button v-if="data?.status==='downloading'" class="text-button" @click="command('update_cancel')">{{t('Cancel Download')}}</button>
      <button v-if="data?.notes" class="text-button" @click="showNotes=!showNotes">{{t('View Release Notes')}}</button></div>
    <div v-if="data?.status==='ready'"><label class="checkbox-label"><input v-model="installConfirmed" type="checkbox"/>{{t('Close messaging browsers, install the verified update and restart')}}</label><button class="primary-button" :disabled="working||!installConfirmed" @click="action('update_install',{confirmed:true})">{{t('Install and Restart')}}</button></div>
    <pre v-if="showNotes&&data?.notes" class="update-notes">{{data.notes}}</pre>
    </template>
  </section>
</template>
<style scoped>.update-notes{white-space:pre-wrap;overflow-wrap:anywhere;font-family:inherit;font-size:13px;line-height:1.6;max-height:300px;overflow:auto}progress{width:100%}.security-actions{flex-wrap:wrap;gap:10px}</style>
