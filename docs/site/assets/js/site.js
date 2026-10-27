// Optional convenience only: navigation and release information work without JS.
for (const button of document.querySelectorAll('[data-copy]')) {
  button.addEventListener('click', async () => {
    const source = document.getElementById(button.dataset.copy);
    const status = document.getElementById('copy-status');
    if (!source || !status) return;
    try {
      await navigator.clipboard.writeText(source.textContent.trim());
      status.textContent = 'Checksum command copied.';
    } catch {
      status.textContent = 'Select and copy the command above.';
    }
  });
}
