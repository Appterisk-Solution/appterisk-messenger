const input = document.getElementById('marker');
function render() {
  document.getElementById('local').textContent = localStorage.getItem('probe-marker') ?? '(empty)';
  document.getElementById('cookie').textContent = document.cookie || '(empty)';
}
document.getElementById('marker-form').addEventListener('submit', event => {
  event.preventDefault();
  localStorage.setItem('probe-marker', input.value);
  document.cookie = `probe-marker=${encodeURIComponent(input.value)}; Max-Age=31536000; Path=/; SameSite=Strict`;
  render();
});
document.getElementById('ipc').addEventListener('click', async () => {
  const result = document.getElementById('result');
  try {
    if (!window.__TAURI_INTERNALS__?.invoke) { result.textContent = 'DENIED: no IPC bridge exposed.'; return; }
    await window.__TAURI_INTERNALS__.invoke('reload_probe', { index: 0 });
    result.textContent = 'FAIL: privileged IPC was accepted. Do not use this build.';
  } catch {
    result.textContent = 'DENIED: privileged command rejected.';
  }
});
render();
