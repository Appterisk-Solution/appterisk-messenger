<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { command, messageFor, type Bounds } from './services/probe';
import { useAccounts, type Account, type Settings } from './stores/accounts';
const Probe=defineAsyncComponent(()=>import('./CompatibilityProbe.vue'));
const store=useAccounts();
const slot=ref<HTMLElement>(),dialogElement=ref<HTMLElement>();
const modal=ref<'add'|'edit'|'remove'|'menu'|'settings'|null>(null);
const tab=ref('general'),target=ref<Account>(),name=ref(''),icon=ref('jade'),notificationPreference=ref(false),legacyId=ref('');
const loaded=ref<Record<string,boolean>>({}),dragged=ref(''),shuttingDown=ref(false),booted=ref(false);
const colors=['jade','blue','violet','rose','amber','slate'];
let observer:ResizeObserver|undefined,timer:ReturnType<typeof setTimeout>|undefined;
let layoutQueue=Promise.resolve();let disposed=false;const stops:UnlistenFn[]=[];
let previousFocus:HTMLElement|null=null;
const systemTheme=window.matchMedia?.('(prefers-color-scheme: dark)');
const settings=computed(()=>store.data?.settings);
const theme=computed(()=>settings.value?.theme??'system');
function applyTheme(){document.documentElement.dataset.theme=theme.value==='system'?(systemTheme?.matches?'dark':'light'):theme.value;document.documentElement.lang=settings.value?.language==='system'?(navigator.language||'en'):settings.value?.language??'en';}
watch(settings,applyTheme,{deep:true});
function bounds():Bounds{const b=slot.value?.getBoundingClientRect();if(!b||b.width<30||b.height<30)throw new Error('Window content is too small. Resize the window and retry.');return{x:b.x,y:b.y,width:b.width,height:b.height};}
function syncLayout(){
  if(!store.ready||store.data?.diagnostic||!slot.value)return Promise.resolve();
  layoutQueue=layoutQueue.catch(()=>{}).then(async()=>{if(!disposed)await command('account_layout',{bounds:bounds(),blocked:!!modal.value});});
  return layoutQueue;
}
function resize(){clearTimeout(timer);timer=setTimeout(()=>{void syncLayout().catch(e=>{store.error=messageFor(e);});},45);}
async function select(id:string){await store.run(async()=>{await layoutQueue.catch(()=>{});await store.select(id,bounds());});}
async function openModal(value:typeof modal.value,account?:Account){
  if(store.busy||shuttingDown.value)return;
  await store.run(async()=>{
    await layoutQueue.catch(()=>{});await command('account_layout',{bounds:bounds(),blocked:true});
    previousFocus=document.activeElement as HTMLElement|null;target.value=account;
    name.value=account?.name??'';icon.value=account?.icon??'jade';notificationPreference.value=account?.notifications_enabled??false;legacyId.value='';
    modal.value=value;await nextTick();dialogElement.value?.querySelector<HTMLElement>('input,button,select')?.focus();
  });
}
async function closeModal(){modal.value=null;await nextTick();await syncLayout();previousFocus?.focus();}
async function saveAccount(){await store.run(async()=>{
  if(modal.value==='edit'&&target.value){await store.mutate('account_update',{id:target.value.id,name:name.value,icon:icon.value,notificationsEnabled:notificationPreference.value});await closeModal();}
  else{const before=new Set(store.data?.accounts.map(a=>a.id));await store.mutate('account_add',{name:name.value,legacyId:legacyId.value||null});const account=store.data?.accounts.find(a=>!before.has(a.id));await closeModal();if(account)await store.select(account.id,bounds());}
});}
async function remove(){if(!target.value)return;const id=target.value.id;await store.run(async()=>{await store.mutate('account_remove',{id,confirmed:true});if(store.activeId===id)store.activeId=null;await closeModal();const next=store.activeId??store.data?.accounts[0]?.id;if(next)await store.select(next,bounds());});}
async function reload(account=store.active){if(!account)return;await store.run(async()=>{loaded.value[account.id]=false;await command('account_reload',{id:account.id});if(modal.value==='menu')await closeModal();});}
async function reorder(from:string,to:string){if(from===to)return;await store.run(()=>store.mutate('account_reorder',{ids:store.reordered(from,to)}));dragged.value='';}
async function move(account:Account,delta:number){const list=store.data?.accounts??[],other=list[list.findIndex(a=>a.id===account.id)+delta];if(other)await reorder(account.id,other.id);}
async function preference(key:keyof Settings,value:string|null){if(!settings.value)return;await store.run(()=>store.mutate('settings_save',{settings:{...settings.value,[key]:value}}));}
async function diagnostics(){await store.run(async()=>{await command('restart_mode',{diagnostic:true});shuttingDown.value=true;});}
function keydown(e:KeyboardEvent){
  if(modal.value){
    if(e.key==='Escape'&&!store.busy){e.preventDefault();void store.run(closeModal);}
    if(e.key==='Tab'){const items=Array.from(dialogElement.value?.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),[tabindex="0"]')??[]);const first=items[0],last=items.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus();}}
    return;
  }
  if(e.ctrlKey&&e.key===','){e.preventDefault();void openModal('settings');}
  if(e.ctrlKey&&e.key.toLowerCase()==='r'){e.preventDefault();void reload();}
  if(e.ctrlKey&&/^[1-9]$/.test(e.key)){const a=store.data?.accounts[Number(e.key)-1];if(a){e.preventDefault();void select(a.id);}}
}
onMounted(async()=>{
  systemTheme?.addEventListener('change',applyTheme);applyTheme();document.addEventListener('keydown',keydown);
  if(isTauri()){
    const loadStop=await listen<{id:string;loaded:boolean}>('account-load',e=>{loaded.value[e.payload.id]=e.payload.loaded;});if(disposed)loadStop();else stops.push(loadStop);
    const shutdownStop=await listen<{state:string;message?:string}>('probe-shutdown',e=>{shuttingDown.value=e.payload.state==='waiting';if(e.payload.state==='blocked')store.error=e.payload.message??'Browser shutdown is pending. Retry closing the window.';});if(disposed)shutdownStop();else stops.push(shutdownStop);
    const shortcutStop=await listen<{key:string}>('account-shortcut',e=>keydown(new KeyboardEvent('keydown',{key:e.payload.key,ctrlKey:true})));if(disposed)shortcutStop();else stops.push(shortcutStop);
    const errorStop=await listen<{id:string;message:string}>('account-error',e=>{if(e.payload.id===store.activeId)store.error=e.payload.message;});if(disposed)errorStop();else stops.push(errorStop);
  }
  await store.run(store.refresh);booted.value=true;await nextTick();
  if(store.data?.diagnostic||store.data?.synthetic)return;
  observer=new ResizeObserver(resize);if(slot.value)observer.observe(slot.value);window.addEventListener('resize',resize);
  const id=store.startupId();if(id)await select(id);
});
onUnmounted(()=>{disposed=true;observer?.disconnect();clearTimeout(timer);stops.forEach(stop=>stop());systemTheme?.removeEventListener('change',applyTheme);window.removeEventListener('resize',resize);document.removeEventListener('keydown',keydown);});
</script>

<template>
  <Probe v-if="store.data?.diagnostic" />
  <div v-else class="messenger" :aria-busy="store.busy || shuttingDown">
    <div class="workspace" :inert="!!modal || undefined">
      <aside class="account-rail" aria-label="WhatsApp accounts">
        <div class="app-mark" title="Appterisk Messenger">✳</div><div class="rail-divider" />
        <nav class="account-list">
          <button v-for="account in store.data?.accounts" :key="account.id" class="account-item" :class="{selected:store.activeId===account.id}" :title="account.name" :aria-label="account.name" :aria-current="store.activeId===account.id?'page':undefined" :disabled="store.busy || shuttingDown" draggable="true" @click="select(account.id)" @contextmenu.prevent="openModal('menu',account)" @dragstart="dragged=account.id" @dragend="dragged=''" @dragover.prevent @drop.prevent="reorder(dragged,account.id)">
            <span class="account-avatar" :class="account.icon">{{ account.name.slice(0,2).toUpperCase() }}<span class="wa-dot">⌕</span></span><span class="rail-name">{{ account.name }}</span>
          </button>
          <button class="add-account" aria-label="Add account" title="Add account" :disabled="store.busy || shuttingDown || !store.ready" @click="openModal('add')">+</button>
        </nav>
        <div class="rail-bottom"><span class="local-dot" title="Profiles stored on this device" /><button class="icon-button settings-button" aria-label="Settings" title="Settings · Ctrl+," :disabled="store.busy || shuttingDown || !store.ready" @click="openModal('settings')">⚙</button></div>
      </aside>
      <main class="conversation-area">
        <header class="topbar">
          <div class="account-heading"><span class="provider-mark">◉</span><div><h1>{{ store.active?.name ?? 'Appterisk Messenger' }}</h1><p>{{ store.active ? 'WhatsApp' : 'Your accounts, together.' }}</p></div></div>
          <div v-if="store.active" class="topbar-actions"><span class="connection-label"><i :class="{'is-loading':!loaded[store.active.id]}" />{{ loaded[store.active.id]?'WhatsApp Web':'Connecting…' }}</span><button class="icon-button" aria-label="Reload account" title="Reload · Ctrl+R" :disabled="store.busy || shuttingDown" @click="reload()">↻</button><button class="icon-button" aria-label="Account options" title="Account options" :disabled="store.busy || shuttingDown" @click="openModal('menu',store.active)">•••</button></div>
          <span v-else class="private-label"><span class="local-dot" /> On your device</span>
        </header>
        <div v-if="store.error && !modal" class="error-banner" role="alert"><span>{{ store.error }}</span><button v-if="!store.ready" :disabled="store.busy" @click="store.run(store.refresh)">Retry</button><button v-else @click="store.error=''" aria-label="Dismiss error">×</button></div>
        <div ref="slot" class="webview-content">
          <div v-if="!store.activeId" class="empty-state">
            <div class="empty-illustration" aria-hidden="true"><svg viewBox="0 0 180 140" fill="none"><rect x="14" y="12" width="152" height="113" rx="24" class="illustration-back"/><path d="M53 47h70a13 13 0 0 1 13 13v28a13 13 0 0 1-13 13H83l-20 15v-15H53a13 13 0 0 1-13-13V60a13 13 0 0 1 13-13Z" class="illustration-chat"/><circle cx="69" cy="75" r="4"/><circle cx="88" cy="75" r="4"/><circle cx="107" cy="75" r="4"/><circle cx="143" cy="31" r="18" class="illustration-badge"/><path d="M143 24v14m-7-7h14" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/></svg></div>
            <p class="eyebrow">A LITTLE MORE CONNECTED</p><h2>{{ !booted?'Getting things ready…':store.data?.accounts.length?'Choose an account':'Make room for every conversation.' }}</h2>
            <p class="empty-copy">{{ store.data?.accounts.length?'Select an account in the sidebar to open WhatsApp.':'Keep your personal and work WhatsApp accounts in one calm space. Each account has its own private session.' }}</p>
            <button class="primary-button" :disabled="store.busy || shuttingDown || !store.ready" @click="openModal('add')"><span>+</span> Add WhatsApp account</button>
            <button v-if="store.data?.legacy.length" class="text-button" :disabled="store.busy" @click="openModal('add')">{{ store.data.legacy.length }} existing profiles available to adopt →</button>
            <div class="privacy-note"><span>▣</span> Local profiles · Official WhatsApp Web · No message database</div>
          </div>
        </div>
        <footer class="statusbar"><span>{{ shuttingDown?'Closing browser profiles safely…':store.busy?'Applying changes…':'APPTERISK' }}</span><span>Independent &amp; unofficial <span class="statusbar-dot">·</span> v{{ store.data?.version??'0.2.0' }}</span></footer>
      </main>
    </div>
    <div v-if="modal" class="modal-backdrop" @click.self="!store.busy && store.run(closeModal)">
      <section ref="dialogElement" class="app-dialog" :class="{'settings-dialog':modal==='settings'}" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
        <template v-if="modal==='settings'">
          <aside class="settings-nav"><p class="eyebrow">APPTERISK</p><h2 id="dialog-title">Settings</h2><button v-for="item in ['general','accounts','about','diagnostics']" :key="item" :class="{active:tab===item}" @click="tab=item">{{ item[0]!.toUpperCase()+item.slice(1) }}</button><p class="settings-version">Version {{ store.data?.version }}</p></aside>
          <div class="settings-body"><button class="close-button" aria-label="Close settings" :disabled="store.busy" @click="store.run(closeModal)">×</button>
            <template v-if="tab==='general'"><p class="eyebrow">MAKE IT YOURS</p><h3>General</h3><p class="section-description">A space that feels right for you.</p>
              <label class="field-label">Appearance</label><div class="theme-options"><button v-for="option in ['light','dark','system'] as const" :key="option" class="theme-option" :class="{chosen:theme===option}" :disabled="store.busy" @click="preference('theme',option)"><span class="theme-preview" :class="option"><i/><b/><b/></span><span>{{ option[0].toUpperCase()+option.slice(1) }}</span></button></div>
              <label class="field-label" for="startup">Open on startup</label><select id="startup" :value="settings?.default_account??''" :disabled="store.busy" @change="preference('default_account',($event.target as HTMLSelectElement).value||null)"><option value="">Last selected account</option><option v-for="a in store.data?.accounts" :key="a.id" :value="a.id">{{ a.name }}</option></select>
              <label class="field-label" for="language">Language preference</label><select id="language" :value="settings?.language" :disabled="store.busy" @change="preference('language',($event.target as HTMLSelectElement).value)"><option value="system">System language</option><option value="en">English</option><option value="ms">Bahasa Melayu</option><option value="zh">中文</option></select><p class="field-help">Saved locally. The shell is currently in English. WhatsApp manages its own language.</p>
              <div class="keyboard-note"><strong>A few useful shortcuts</strong><span>Ctrl + 1–9 <small>Switch account</small></span><span>Ctrl + R <small>Reload account</small></span><span>Ctrl + , <small>Open settings</small></span></div>
            </template>
            <template v-else-if="tab==='accounts'"><p class="eyebrow">YOUR CONNECTIONS</p><h3>Accounts</h3><p class="section-description">Drag accounts in the sidebar to reorder them.</p><div class="settings-accounts"><div v-for="a in store.data?.accounts" :key="a.id" class="settings-account"><span class="small-avatar" :class="a.icon">{{ a.name.slice(0,2).toUpperCase() }}</span><span class="account-description"><strong>{{ a.name }}</strong><small>WhatsApp · Separate profile</small></span><button class="icon-button" :disabled="store.busy || a.id===store.data?.accounts[0]?.id" :aria-label="`Move ${a.name} up`" @click="move(a,-1)">↑</button><button class="icon-button" :disabled="store.busy || a.id===store.data?.accounts.at(-1)?.id" :aria-label="`Move ${a.name} down`" @click="move(a,1)">↓</button><button class="text-button" :disabled="store.busy" @click="modal='edit';target=a;name=a.name;icon=a.icon;notificationPreference=a.notifications_enabled">Edit</button></div></div><button class="secondary-button mt-5" :disabled="store.busy" @click="modal='add';name='';legacyId=''">+ Add account</button></template>
            <template v-else-if="tab==='about'"><div class="about-mark">✳</div><h3>Appterisk Messenger</h3><p class="section-description">Your accounts, together.</p><dl class="about-details"><dt>Application</dt><dd>{{ store.data?.version }}</dd><dt>WebView2 Evergreen</dt><dd>{{ store.data?.runtime_version??'Not available' }}</dd><dt>Storage</dt><dd>Local device only</dd></dl><p class="field-help">An independent application displaying the official WhatsApp Web interface. Not affiliated with WhatsApp or Meta.</p><p class="field-help">Notifications, unread counts, background delivery, calls, media and downloads have not been verified for this release. New sensitive permission requests and downloads are currently denied.</p></template>
            <template v-else><p class="eyebrow">DEVELOPER TOOLS</p><h3>Diagnostics</h3><p class="section-description">The original compatibility probe is still available.</p><div class="diagnostic-note"><strong>Open a separate diagnostic mode</strong><p>Appterisk will close browsers safely and reopen with the three-profile probe. Your account metadata and profiles are preserved. Close other Appterisk windows first.</p><button class="secondary-button" :disabled="store.busy" @click="diagnostics">Restart in diagnostic mode</button></div><p class="field-help">Storage diagnostics collect counts and runtime metadata. They do not export messages or authentication data.</p></template>
          </div>
        </template>
        <template v-else>
          <button class="close-button" aria-label="Close dialog" :disabled="store.busy" @click="store.run(closeModal)">×</button>
          <template v-if="modal==='add'||modal==='edit'"><p class="eyebrow">{{ modal==='add'?'A NEW CONNECTION':'ACCOUNT DETAILS' }}</p><h2 id="dialog-title">{{ modal==='add'?'Add WhatsApp account':'Make it your own' }}</h2><p class="section-description">{{ modal==='add'?'Give this account a name. You’ll link it in the official WhatsApp interface.':'A familiar name makes switching easier.' }}</p>
            <form @submit.prevent="saveAccount"><template v-if="modal==='add' && store.data?.legacy.length"><label class="field-label" for="profile">Browser profile</label><select id="profile" v-model="legacyId" :disabled="store.busy" @change="!name && legacyId && (name=store.data?.legacy.find(a=>a.id===legacyId)?.name??'')"><option value="">Create a new, separate account</option><option v-for="a in store.data.legacy" :key="a.id" :value="a.id">Adopt {{ a.name }}</option></select><p class="field-help">Adoption reuses the existing profile in place. Login databases are never copied or moved.</p></template>
              <label class="field-label" for="account-name">Account name</label><input id="account-name" v-model="name" required maxlength="64" placeholder="Personal, Work, Family…" autocomplete="off" :disabled="store.busy" />
              <template v-if="modal==='edit'"><span class="field-label">Account color</span><div class="color-options"><button v-for="c in colors" :key="c" type="button" :class="[c,{chosen:icon===c}]" :aria-label="c" :aria-pressed="icon===c" :disabled="store.busy" @click="icon=c">{{ icon===c?'✓':'' }}</button></div><label class="checkbox-label"><input v-model="notificationPreference" type="checkbox" :disabled="store.busy" /> Save notification preference</label><p class="field-help">Preference only. Notification delivery is not enabled or verified in this release.</p></template>
              <div class="dialog-actions"><button type="button" class="secondary-button" :disabled="store.busy" @click="store.run(closeModal)">Cancel</button><button class="primary-button" :disabled="store.busy || !name.trim()">{{ modal==='edit'?'Save changes':legacyId?'Adopt account':'Add account' }}</button></div>
            </form>
          </template>
          <template v-else-if="modal==='remove'"><div class="warning-icon">−</div><h2 id="dialog-title">Remove {{ target?.name }}?</h2><p class="section-description">This removes the account from your sidebar. Its browser profile stays on this device. Other accounts are unaffected.</p><p class="field-help">Removing an account does not log it out of WhatsApp. Use WhatsApp → Linked devices on your phone to unlink a device.</p><div class="dialog-actions"><button class="secondary-button" :disabled="store.busy" @click="store.run(closeModal)">Keep account</button><button class="danger-button" :disabled="store.busy" @click="remove">Remove account</button></div></template>
          <template v-else-if="modal==='menu'"><p class="eyebrow">WHATSAPP ACCOUNT</p><h2 id="dialog-title">{{ target?.name }}</h2><div class="context-actions"><button @click="modal='edit'">✎ <span>Rename &amp; customize</span></button><button :disabled="store.busy || target?.id!==store.activeId" @click="reload(target)">↻ <span>Reload account</span></button><button :disabled="store.busy" @click="target && move(target,-1)">↑ <span>Move up</span></button><button :disabled="store.busy" @click="target && move(target,1)">↓ <span>Move down</span></button><button class="danger-text" @click="modal='remove'">− <span>Remove account…</span></button></div></template>
        </template>
        <p v-if="store.error" class="dialog-error" role="alert">{{ store.error }}</p>
      </section>
    </div>
  </div>
</template>
