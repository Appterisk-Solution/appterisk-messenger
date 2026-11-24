// Runs only in the opt-in local diagnostic fixture, never on provider content.
window.addEventListener('DOMContentLoaded', async () => {
  const config = window.__isolationConfig;
  const open = () => new Promise((resolve, reject) => {
    const request = indexedDB.open('appterisk-isolation', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('markers');
    request.onerror = () => reject(new Error('IndexedDB open failed'));
    request.onsuccess = () => resolve(request.result);
  });
  const value = async (write, key = 'identity', payload = config.marker) => {
    const db = await open();
    try {
      return await new Promise((resolve, reject) => {
        const transaction = db.transaction('markers', write ? 'readwrite' : 'readonly');
        const store = transaction.objectStore('markers');
        const request = write ? store.put(payload, key) : store.get(key);
        let result;
        request.onsuccess = () => { result = request.result; };
        transaction.oncomplete = () => resolve(result ?? null);
        transaction.onerror = () => reject(new Error('IndexedDB transaction failed'));
        transaction.onabort = () => reject(new Error('IndexedDB transaction aborted'));
      });
    } finally { db.close(); }
  };
  let ipcDenied = false;
  const verifyRichRecords = async () => {
    const blob = await value(false, 'synthetic-blob');
    const encrypted = await value(false, 'synthetic-crypto');
    const decoded = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: encrypted.iv }, encrypted.key, encrypted.ciphertext);
    return {
      blobOk: blob instanceof Blob && await blob.text() === config.marker.repeat(4096),
      cryptoOk: encrypted.key.extractable === false && new TextDecoder().decode(decoded) === config.marker,
    };
  };
  window.__readIsolation = async () => {
    window.__isolationResult = null;
    try {
      const rich = await verifyRichRecords();
      window.__isolationResult = {
        local: localStorage.getItem('isolation_marker'),
        cookie: document.cookie.split('; ').find(item => item.startsWith('isolation_marker='))?.slice('isolation_marker='.length) ?? null,
        indexedDb: await value(false),
        ipcDenied,
        ...rich,
        sessionCookiePresent: document.cookie.split('; ').some(item => item.startsWith('fixture_session=')),
      };
    } catch { window.__isolationResult = { error: 'Storage read failed' }; }
  };
  try {
    if (config.stage === 'seed') {
      if (localStorage.getItem('isolation_marker') !== null || await value(false) !== null || document.cookie.includes('isolation_marker=')) {
        throw new Error('Fresh profile contained another marker');
      }
      localStorage.setItem('isolation_marker', config.marker);
      document.cookie = `isolation_marker=${config.marker}; Max-Age=31536000; Path=/; SameSite=Strict`;
      document.cookie = `fixture_session=${config.marker}; Path=/; SameSite=Strict`;
      await value(true);
      await value(true, 'synthetic-blob', new Blob([config.marker.repeat(4096)]));
      // Fixture-only random key; never exported or used for any real authentication.
      const key = await crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt']);
      const iv = crypto.getRandomValues(new Uint8Array(12));
      const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, new TextEncoder().encode(config.marker));
      await value(true, 'synthetic-crypto', { key, iv, ciphertext });
    }
    if (!window.__TAURI_INTERNALS__?.invoke) ipcDenied = true;
    else {
      try { await window.__TAURI_INTERNALS__.invoke('reload_probe', { index: 0 }); }
      catch (error) {
        ipcDenied = error?.code === 'FORBIDDEN'
          || (typeof error === 'string' && /not allowed|not permitted|not authorized/i.test(error));
        if (!ipcDenied) throw new Error('IPC failed without an authorization denial', { cause: error });
      }
    }
    await window.__readIsolation();
  } catch (error) { window.__isolationResult = { error: error.message }; }
});
