<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue';
import { boundsFor, command, messageFor } from './services/probe';
import { isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import './probe.css';
const panels = ref<HTMLElement[]>([]);
const started = ref(false);
const busy = ref(false);
const help = ref(false);
const hidden = ref(false);
const error = ref('');
const mode = ref('');
let observer: ResizeObserver | undefined;
let timer: ReturnType<typeof setTimeout> | undefined;
let unlisten:UnlistenFn|undefined;
let mounted=true;
async function perform(action: () => Promise<void>) {
  if (busy.value) return;
  busy.value = true; error.value = '';
  try { await action(); } catch (cause) { error.value = messageFor(cause); }
  finally { busy.value = false; }
}
async function open(fixture: boolean) {
  await perform(async () => {
    started.value = false;
    await nextTick();
    await command('open_probe', { bounds: boundsFor(panels.value), fixture });
    started.value = true; hidden.value = false;
    mode.value = fixture ? 'Local isolation fixture' : 'Official WhatsApp Web';
  });
}
async function toggleVisibility() {
  await perform(async () => {
    await command('set_probe_visibility', { visible: hidden.value });
    hidden.value = !hidden.value;
  });
}
async function toggleHelp() {
  await perform(async () => {
    await command('set_probe_visibility', { visible: help.value && !hidden.value });
    help.value = !help.value;
    await nextTick();
    document.getElementById(help.value ? 'close-help' : 'help-button')?.focus();
  });
}
function resize() {
  clearTimeout(timer);
  timer = setTimeout(async () => {
    if (!started.value) return;
    try { await command('set_probe_bounds', { bounds: boundsFor(panels.value) }); }
    catch (cause) { error.value = messageFor(cause); }
  }, 60);
}
function keys(event: KeyboardEvent) {
  if (!help.value) return;
  if (event.key === 'Escape' && !busy.value) void toggleHelp();
  if (event.key === 'Tab') { event.preventDefault(); document.getElementById('close-help')?.focus(); }
}
onMounted(() => {
  observer = new ResizeObserver(resize);
  panels.value.forEach(panel => observer?.observe(panel));
  window.addEventListener('resize', resize); document.addEventListener('keydown', keys);
  if (isTauri()) void listen<{state:string;message?:string}>('probe-shutdown',event=>{
    busy.value=event.payload.state==='waiting';
    if (event.payload.state==='blocked') {
      started.value=false;
      error.value=event.payload.message ?? 'Browser shutdown is still pending. Retry closing the app.';
    }
  }).then(stop=>{ if (mounted) unlisten=stop; else stop(); });
  if (isTauri()) void command<boolean>('resume_probe_on_startup').then(resume=>{
    if (mounted && resume) void open(false);
  }).catch(cause=>{error.value=messageFor(cause);});
});
onUnmounted(() => {
  clearTimeout(timer); observer?.disconnect();
  mounted=false; unlisten?.();
  window.removeEventListener('resize', resize); document.removeEventListener('keydown', keys);
});
</script>
<template>
  <div class="shell" :inert="help || undefined">
    <aside class="rail" aria-label="Appterisk"><span class="brand">✳</span><span>LAB</span><span class="bottom">01</span></aside>
    <main>
      <header><div><p class="eyebrow">APPTERISK MESSENGER</p><h1>Three profiles. One window.</h1><p class="subtitle">Compatibility lab · Windows / WebView2</p></div><button :disabled="busy" @click="perform(() => command('restart_mode',{diagnostic:false}))">Return to account manager</button></header>
      <section class="intro"><div><h2>Start with isolation.</h2><p>Each panel uses a separate persistent browser profile. Test local storage first, then link three different numbers in the official WhatsApp interface.</p></div><div class="actions"><button :disabled="busy" @click="open(true)">Test local isolation</button><button class="primary" :disabled="busy" @click="open(false)">Open WhatsApp Web ↗</button></div></section>
      <div class="notice"><strong>Optional developer diagnostics</strong><span>Three-account persistence was user-confirmed on Windows in 0.1.2. These tools inspect compatibility; other platforms remain unverified.</span></div>
      <div class="tools"><span aria-live="polite">{{ busy ? 'Applying changes or closing browser profiles safely…' : started ? mode : 'Choose a test to open the native panels' }}</span><div><button :disabled="busy || !started || mode !== 'Official WhatsApp Web'" @click="perform(() => command('capture_profile_metadata'))">Capture storage metadata</button><button :disabled="busy || !started" @click="toggleVisibility">{{ hidden ? 'Show panels' : 'Hide panels' }}</button><button id="help-button" :disabled="busy" @click="toggleHelp">Test guide</button></div></div>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <section class="panels" aria-label="Isolated browser panels"><article v-for="index in 3" :key="index"><div class="panel-heading"><span class="avatar" :class="`color-${index}`">0{{ index }}</span><div><h2>Profile {{ index }}</h2><p>Persistent · independent directory</p></div><button :aria-label="`Reload profile ${index}`" :disabled="busy || !started" @click="perform(() => command('reload_probe', { index: index - 1 }))">↻</button></div><div :ref="el => { if (el) panels[index - 1] = el as HTMLElement }" class="native-slot"><div v-if="!started || hidden" class="placeholder"><span>◈</span><h3>{{ hidden ? 'Panel hidden' : 'Ready to test' }}</h3><p>{{ hidden ? 'The browser session remains alive.' : 'The genuine native WebView will appear here.' }}</p></div></div></article></section>
      <footer><span>Local profiles. No message database. No telemetry.</span><span>Independent & unofficial · v0.2.0 diagnostics</span></footer>
    </main>
  </div>
  <div v-if="help" class="backdrop"><section class="dialog" role="dialog" aria-modal="true" aria-labelledby="guide-title"><p class="eyebrow">MANUAL COMPATIBILITY GATE</p><h2 id="guide-title">Verify before building further</h2><ol><li>Open the local fixture. Write different markers and cookies in all three panels.</li><li>Reload and restart the app. Each panel must retain only its own markers.</li><li>Run the IPC-denial test in every fixture panel. It must report denied.</li><li>Open WhatsApp Web. Link three different numbers, then restart and confirm independent sessions.</li><li>Resize, move across DPI displays, hide/show panels and reopen this guide.</li><li>Record browser rejection and media restrictions. Downloads, popups, camera, microphone and notifications are deliberately denied in this probe.</li></ol><p>Tests are pending. See docs/COMPATIBILITY.md for the full acceptance record.</p><button id="close-help" class="primary" :disabled="busy" @click="toggleHelp">Back to profiles</button><p v-if="error" class="error" role="alert">{{ error }}</p></section></div>
</template>
