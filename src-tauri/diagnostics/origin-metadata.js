// Explicitly requested, read-only browser API metadata; never read keys, values, messages or QR codes.
void (async () => {
  if (location.origin !== 'https://web.whatsapp.com') return;
  window.__appteriskStorageMetadata = null;
  const count = async action => { try { return (await action()).length; } catch { return null; } };
  let localStorageEntries = null;
  let persistentStorage = null;
  let activeServiceWorkers = null;
  let quotaUsageBytes = null;
  let quotaLimitBytes = null;
  try {
    const estimate = await navigator.storage.estimate();
    if (Number.isSafeInteger(estimate.usage) && estimate.usage >= 0) quotaUsageBytes = estimate.usage;
    if (Number.isSafeInteger(estimate.quota) && estimate.quota >= 0) quotaLimitBytes = estimate.quota;
  } catch { /* aggregate quota metadata unavailable */ }
  try { localStorageEntries = localStorage.length; } catch { /* unavailable */ }
  try { persistentStorage = await navigator.storage.persisted(); } catch { /* unavailable */ }
  const serviceWorkerRegistrations = await count(async () => {
    const registrations = await navigator.serviceWorker.getRegistrations();
    activeServiceWorkers = registrations.filter(registration => registration.active !== null).length;
    return registrations;
  });
  window.__appteriskStorageMetadata = {
    localStorageEntries,
    indexedDbDatabases: await count(() => indexedDB.databases()),
    serviceWorkerRegistrations,
    activeServiceWorkers,
    cacheStorageCaches: await count(() => caches.keys()),
    persistentStorage,
    scriptErrors: window.__appteriskErrorCounts?.scriptErrors ?? null,
    unhandledRejections: window.__appteriskErrorCounts?.unhandledRejections ?? null,
    quotaUsageBytes,
    quotaLimitBytes,
  };
})();
