<script setup lang="ts">
import {ref} from 'vue';
import {useDesktop,type DesktopConfig} from '../stores/desktop';
import {useAccounts} from '../stores/accounts';
import {command} from '../services/probe';
import {t} from '../i18n';
defineProps<{tab:string}>();
const desktop=useDesktop(),accounts=useAccounts();
const currentPassword=ref(''),newPassword=ref(''),confirmation=ref(''),permissionAccount=ref('');
const permissionKinds=['notifications','microphone','camera','clipboard','autoplay'];
async function preference<K extends keyof DesktopConfig>(key:K,value:DesktopConfig[K]){await accounts.run(()=>desktop.preference(key,value));}
async function accountNotification(id:string,enabled:boolean){const a=accounts.data?.accounts.find(a=>a.id===id);if(a)await accounts.run(()=>accounts.mutate('account_update',{id,name:a.name,icon:a.icon,notificationsEnabled:enabled}));}
async function folder(){await accounts.run(async()=>{const directory=await command<string|null>('desktop_download_directory');if(directory)await desktop.preference('download_directory',directory);});}
async function password(disable=false){await accounts.run(async()=>{try{if(!disable&&newPassword.value!==confirmation.value)throw new Error(t('Passwords do not match.'));await command('desktop_password',{currentPassword:currentPassword.value,newPassword:disable?null:newPassword.value});await desktop.refresh();}finally{currentPassword.value='';newPassword.value='';confirmation.value='';}});}
function decision(kind:string){return desktop.data?.permissions.find(p=>p.account_id===permissionAccount.value&&p.kind===kind)?.decision??'ask';}
async function permission(kind:string,value:string){if(permissionAccount.value)await accounts.run(async()=>{await command('desktop_permission',{accountId:permissionAccount.value,kind,decision:value});await desktop.refresh();});}
function label(kind:string){return t(({notifications:'Notifications',microphone:'Microphone',camera:'Camera',clipboard:'Clipboard read',autoplay:'Audio playback'} as Record<string,string>)[kind]??kind);}
</script>
<template>
  <section v-if="desktop.data">
    <template v-if="tab==='notifications'"><p class="eyebrow">{{ t('STAY IN THE LOOP') }}</p><h3>{{ t('Notifications') }}</h3><p class="section-description">{{ t('Native notifications are routed to their original account.') }}</p>
      <label class="setting-toggle"><span>{{ t('Enable desktop notifications') }}</span><input type="checkbox" :checked="desktop.data.config.notifications" :disabled="accounts.busy" @change="preference('notifications',($event.target as HTMLInputElement).checked)" /></label>
      <label class="setting-toggle"><span>{{ t('Do Not Disturb') }}</span><input type="checkbox" :checked="desktop.data.config.dnd" :disabled="accounts.busy" @change="preference('dnd',($event.target as HTMLInputElement).checked)" /></label>
      <label class="setting-toggle"><span>{{ t('Notification sound') }}</span><input type="checkbox" :checked="desktop.data.config.sound" :disabled="accounts.busy" @change="preference('sound',($event.target as HTMLInputElement).checked)" /></label>
      <label class="setting-toggle"><span>{{ t('Hide message previews') }}</span><input type="checkbox" :checked="desktop.data.config.hide_previews" :disabled="accounts.busy" @change="preference('hide_previews',($event.target as HTMLInputElement).checked)" /></label>
      <h4 class="field-label">{{ t('Per-account notifications') }}</h4><label v-for="a in accounts.data?.accounts" :key="a.id" class="setting-toggle"><span>{{ a.name }}</span><input type="checkbox" :checked="a.notifications_enabled" :disabled="accounts.busy" @change="accountNotification(a.id,($event.target as HTMLInputElement).checked)" /></label>
      <p class="field-help">{{ t('WhatsApp must also be allowed to send notifications. Grant permission when prompted or use Privacy and Security. Windows notification settings and Focus Assist still apply.') }}</p>
      <p class="field-help">{{ t('Unread numbers come from the WhatsApp window title and may count chats rather than messages. A dot means unread activity without a reliable count.') }}</p>
    </template>
    <template v-else-if="tab==='tray'"><p class="eyebrow">{{ t('KEEP IT CLOSE') }}</p><h3>{{ t('System Tray') }}</h3><p class="section-description">{{ t('Initialized accounts remain alive while the window is hidden. Quit always closes browsers safely.') }}</p>
      <label class="setting-toggle"><span>{{ t('Minimize to system tray') }}</span><input type="checkbox" :checked="desktop.data.config.minimize_to_tray" :disabled="accounts.busy" @change="preference('minimize_to_tray',($event.target as HTMLInputElement).checked)" /></label>
      <label class="setting-toggle"><span>{{ t('Close to system tray') }}</span><input type="checkbox" :checked="desktop.data.config.close_to_tray" :disabled="accounts.busy" @change="preference('close_to_tray',($event.target as HTMLInputElement).checked)" /></label><p class="field-help">{{ t('Off by default: X exits the application. When enabled, use Quit in the tray menu to exit.') }}</p>
      <label class="setting-toggle"><span>{{ t('Start minimized') }}</span><input type="checkbox" :checked="desktop.data.config.start_minimized" :disabled="accounts.busy" @change="preference('start_minimized',($event.target as HTMLInputElement).checked)" /></label>
    </template>
    <template v-else-if="tab==='downloads'"><p class="eyebrow">{{ t('FILES ON YOUR TERMS') }}</p><h3>{{ t('Downloads') }}</h3><p class="section-description">{{ t('Files use the native save dialog and are never opened automatically. Existing files are not silently overwritten.') }}</p>
      <label class="setting-toggle"><span>{{ t('Ask where to save') }}</span><input type="checkbox" :checked="desktop.data.config.ask_before_saving" :disabled="accounts.busy" @change="preference('ask_before_saving',($event.target as HTMLInputElement).checked)" /></label>
      <label class="field-label">{{ t('Download directory') }}</label><p class="download-path">{{ desktop.data.config.download_directory??t('Windows Downloads folder') }}</p><button class="secondary-button" :disabled="accounts.busy" @click="folder">{{ t('Choose folder…') }}</button>
      <h4 class="field-label">{{ t('Recent downloads') }}</h4><p v-if="!desktop.data.downloads.length" class="field-help">{{ t('No downloads in this session.') }}</p>
      <article v-for="download in desktop.data.downloads" :key="download.id" class="download-item"><strong>{{ download.name }}</strong><small>{{ accounts.data?.accounts.find(a=>a.id===download.account_id)?.name }} · {{ t(download.state) }}</small><progress v-if="download.state==='downloading'" :value="download.total?download.received:undefined" :max="download.total??1"/><p v-if="download.saved_path" class="download-path">{{ download.saved_path }}</p><button v-if="['choosing','downloading'].includes(download.state)" class="text-button" :disabled="accounts.busy" @click="accounts.run(()=>command('desktop_download_cancel',{id:download.id}))">{{ t('Cancel') }}</button></article>
      <p class="field-help">{{ t('Uploads use the official WhatsApp attachment controls and the native WebView2 file picker. No general filesystem IPC is granted to WhatsApp.') }}</p>
    </template>
    <template v-else-if="tab==='privacy'"><p class="eyebrow">{{ t('YOUR DEVICE, YOUR CHOICE') }}</p><h3>{{ t('Privacy and Security') }}</h3><p class="section-description">{{ t('The local lock hides the application. It does not encrypt WhatsApp browser profiles or protect against another Windows administrator.') }}</p>
      <form @submit.prevent="password(false)"><label v-if="desktop.data.lock_enabled" class="field-label" for="current-password">{{ t('Current password') }}</label><input v-if="desktop.data.lock_enabled" id="current-password" v-model="currentPassword" type="password" autocomplete="current-password" maxlength="1024" :disabled="accounts.busy" />
        <label class="field-label" for="new-password">{{ t('New password') }}</label><input id="new-password" v-model="newPassword" type="password" autocomplete="new-password" minlength="8" maxlength="1024" required :disabled="accounts.busy" /><label class="field-label" for="confirm-password">{{ t('Confirm password') }}</label><input id="confirm-password" v-model="confirmation" type="password" autocomplete="new-password" required maxlength="1024" :disabled="accounts.busy" />
        <div class="security-actions"><button class="secondary-button" :disabled="accounts.busy">{{ desktop.data.lock_enabled?t('Change lock password'):t('Enable application lock') }}</button><button v-if="desktop.data.lock_enabled" type="button" class="text-button danger-text" :disabled="accounts.busy || !currentPassword" @click="password(true)">{{ t('Disable lock') }}</button></div>
      </form>
      <label class="field-label" for="lock-timeout">{{ t('Automatically lock after inactivity') }}</label><select id="lock-timeout" :value="desktop.data.config.lock_timeout_seconds" :disabled="accounts.busy" @change="preference('lock_timeout_seconds',Number(($event.target as HTMLSelectElement).value))"><option :value="0">{{ t('Never') }}</option><option :value="60">{{ t('1 minute') }}</option><option :value="300">{{ t('5 minutes') }}</option><option :value="600">{{ t('10 minutes') }}</option><option :value="900">{{ t('15 minutes') }}</option><option :value="1800">{{ t('30 minutes') }}</option></select>
      <h4 class="field-label">{{ t('Origin-scoped permissions') }}</h4><p class="field-help">{{ t('Only https://web.whatsapp.com can request these permissions. Changes reload that account to stop active capture. Unknown origins and permission types are denied.') }}</p>
      <select v-model="permissionAccount" class="permission-account" :disabled="accounts.busy"><option value="">{{ t('Choose an account') }}</option><option v-for="a in accounts.data?.accounts" :key="a.id" :value="a.id">{{ a.name }}</option></select>
      <label v-for="kind in permissionKinds" v-show="permissionAccount" :key="kind" class="permission-row"><span>{{ label(kind) }}</span><select :value="decision(kind)" :disabled="accounts.busy" @change="permission(kind,($event.target as HTMLSelectElement).value)"><option value="ask">{{ t('Ask') }}</option><option value="allow">{{ t('Allow') }}</option><option value="deny">{{ t('Deny') }}</option></select></label>
      <p class="field-help">{{ t('Windows camera and microphone permissions also apply. Voice messages and calls require real-device verification; no calling compatibility is assumed.') }}</p>
    </template>
  </section>
</template>
