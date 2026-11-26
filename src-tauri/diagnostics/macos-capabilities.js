/* global sessionStorage */
// Opt-in troubleshooting only. No page text, messages, credentials, QR data,
// provider records, error messages or stacks are collected. Only this script's
// own diagnostic session key is read and written.
(() => {
  const allowed=['clone','quota','crypto','indexeddb','worker','notification','network','unsupported','type','other'];
  let errors=[];
  try { const old=JSON.parse(sessionStorage.getItem('__appteriskDiagnosticKinds')||'[]'); if(Array.isArray(old))errors=old.filter(x=>allowed.includes(x)).slice(-20); } catch { /* Storage can be unavailable; diagnostics remain optional. */ }
  const record=event=>{
    const error=event.error||event.reason;
    const name=typeof error?.name==='string'?error.name:'';
    const message=typeof error?.message==='string'?error.message:'';
    let kind='other';
    if(name==='DataCloneError')kind='clone';
    else if(name==='QuotaExceededError')kind='quota';
    else if(/crypto|keychain|encrypt|decrypt/i.test(message))kind='crypto';
    else if(/indexeddb|idbfactory|idbtransaction/i.test(message))kind='indexeddb';
    else if(/serviceworker|service worker|sharedworker/i.test(message))kind='worker';
    else if(/notification/i.test(message))kind='notification';
    else if(/websocket|network|failed to fetch/i.test(message))kind='network';
    else if(name==='NotSupportedError')kind='unsupported';
    else if(name==='TypeError')kind='type';
    errors.push(kind);errors=errors.slice(-20);
    try{sessionStorage.setItem('__appteriskDiagnosticKinds',JSON.stringify(errors));}catch{ /* Storage can be unavailable; diagnostics remain optional. */ }
  };
  window.addEventListener('error',record);
  window.addEventListener('unhandledrejection',record);
  window.__appteriskCapabilitySnapshot=()=>({
    secure:window.isSecureContext,
    crypto:!!globalThis.crypto?.subtle,
    indexeddb:typeof indexedDB==='object',
    worker:typeof Worker==='function',
    serviceworker:'serviceWorker' in navigator,
    controlled:!!navigator.serviceWorker?.controller,
    notification:typeof Notification==='function',
    wasm:typeof WebAssembly==='object',
    sharedarray:typeof SharedArrayBuffer==='function',
    errors
  });
})();
