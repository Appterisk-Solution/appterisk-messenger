<script setup lang="ts">
import {ref,onMounted,onUnmounted} from 'vue';
import {useDesktop,type Prompt} from '../stores/desktop';
import {useAccounts} from '../stores/accounts';
import {command} from '../services/probe';
import {t} from '../i18n';
const props=defineProps<{prompt:Prompt}>();const remember=ref(false),dialog=ref<HTMLElement>();const accounts=useAccounts(),desktop=useDesktop();
async function answer(allow:boolean){await accounts.run(async()=>{await command('desktop_permission_reply',{id:props.prompt.id,allow,remember:remember.value});await desktop.refresh();await accounts.refresh();});}
function key(e:KeyboardEvent){if(e.key==='Escape'){e.preventDefault();void answer(false);}if(e.key==='Tab'){const items=Array.from(dialog.value?.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled)')??[]);if(e.shiftKey&&document.activeElement===items[0]){e.preventDefault();items.at(-1)?.focus();}else if(!e.shiftKey&&document.activeElement===items.at(-1)){e.preventDefault();items[0]?.focus();}}}
onMounted(()=>{dialog.value?.querySelector<HTMLElement>('button')?.focus();document.addEventListener('keydown',key);});onUnmounted(()=>document.removeEventListener('keydown',key));
</script>
<template><div class="modal-backdrop permission-backdrop"><section ref="dialog" class="app-dialog" role="dialog" aria-modal="true" aria-labelledby="permission-title"><p class="eyebrow">{{ t('PERMISSION REQUEST') }}</p><h2 id="permission-title">{{ t('Allow access?') }}</h2><p class="section-description">{{ accounts.data?.accounts.find(a=>a.id===prompt.account_id)?.name }} · {{ t(({microphone:'Microphone',camera:'Camera',notifications:'Notifications',clipboard:'Clipboard read',autoplay:'Audio playback'} as Record<string,string>)[prompt.kind]??prompt.kind) }}</p><p class="download-path">{{ prompt.origin }}</p><p class="field-help">{{ t('Approve only if you intended this action. This request expires after one minute.') }}</p><label class="checkbox-label"><input v-model="remember" type="checkbox" :disabled="accounts.busy"/>{{ t('Remember for this account and origin') }}</label><div class="dialog-actions"><button class="secondary-button" :disabled="accounts.busy" @click="answer(false)">{{ t('Deny') }}</button><button class="primary-button" :disabled="accounts.busy" @click="answer(true)">{{ t('Allow') }}</button></div></section></div></template>
