// Categorize errors without reading messages, stack traces, URLs, resources or provider data.
if (location.origin === 'https://web.whatsapp.com') {
  window.__appteriskErrorCounts = { scriptErrors: 0, unhandledRejections: 0 };
  window.addEventListener('error', () => {
    window.__appteriskErrorCounts.scriptErrors = Math.min(1000000, window.__appteriskErrorCounts.scriptErrors + 1);
  });
  window.addEventListener('unhandledrejection', () => {
    window.__appteriskErrorCounts.unhandledRejections = Math.min(1000000, window.__appteriskErrorCounts.unhandledRejections + 1);
  });
}
