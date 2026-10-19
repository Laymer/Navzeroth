import assert from 'node:assert/strict';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const root = path.join(repo, 'site');
const pages = ['index.html', 'features.html', 'installation.html', 'readiness.html', 'code-signing.html'];
const draft = process.argv.includes('--draft');
const require = createRequire(path.join(repo, 'apps/desktop/package.json'));
const { chromium } = require('@playwright/test');
const hash = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
let release;
if (!draft) {
  release = JSON.parse(fs.readFileSync(path.join(root, 'release.json')));
  assert.equal(release.tag, 'v1.0.0-rc.1');
  assert.equal(release.public_verified, true);
  assert.match(release.source_commit, /^[a-f0-9]{40}$/);
  assert.equal(release.assets.length, 5);
  for (const asset of release.assets) {
    assert.match(asset.sha256, /^[a-f0-9]{64}$/);
    assert.equal(asset.url, `https://github.com/Laymer/Navzeroth/releases/download/${release.tag}/${asset.name}`);
  }
  const capture = JSON.parse(fs.readFileSync(path.join(root, 'assets/screenshots/provenance.json')));
  assert.equal(capture.release_tag, release.tag);
  assert.equal(capture.source_commit, release.source_commit);
  assert.equal(capture.installer_sha256, release.assets.find(a => a.name.endsWith('-setup.exe')).sha256);
  assert.equal(capture.synthetic, true);
  assert.match(capture.workflow_run_id, /^\d+$/);
  assert.match(capture.application_sha256, /^[a-f0-9]{64}$/);
  assert.ok(capture.screenshots.length >= 2);
  for (const screenshot of capture.screenshots) {
    assert.match(screenshot.file, /^[A-Za-z0-9-]+\.png$/);
    assert.equal(hash(path.join(root, 'assets/screenshots', screenshot.file)), screenshot.sha256);
    assert.ok(screenshot.width >= 640 && screenshot.height >= 480);
  }
}
const mime = { '.html':'text/html', '.css':'text/css', '.js':'text/javascript', '.json':'application/json', '.png':'image/png', '.woff2':'font/woff2' };
const server = http.createServer((request, response) => {
  const url = new URL(request.url, 'http://localhost');
  if (!url.pathname.startsWith('/Navzeroth/')) { response.writeHead(404).end(); return; }
  const relative = decodeURIComponent(url.pathname.slice('/Navzeroth/'.length)) || 'index.html';
  const file = path.resolve(root, relative);
  if (!file.startsWith(root + path.sep) || !fs.existsSync(file) || !fs.statSync(file).isFile()) { response.writeHead(404).end(); return; }
  response.setHeader('Content-Type', mime[path.extname(file)] ?? 'application/octet-stream');
  response.end(fs.readFileSync(file));
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
  browser = await chromium.launch({ headless: true });
  const origin = `http://127.0.0.1:${server.address().port}`;
  const context = await browser.newContext({ reducedMotion:'reduce', permissions:['clipboard-read','clipboard-write'] });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
  for (const width of [360, 768, 1440]) {
    await page.setViewportSize({ width, height:900 });
    for (const file of pages) {
      await page.goto(`${origin}/Navzeroth/${file}`);
      await page.evaluate(() => document.fonts.ready);
      assert.equal(await page.locator('html').getAttribute('lang'), 'en');
      assert.equal(await page.locator('h1').count(), 1);
      assert.equal(await page.getByRole('main').count(), 1);
      assert.equal(await page.getByRole('navigation', { name:'Main navigation' }).count(), 1);
      assert.equal(await page.locator('nav [aria-current="page"]').count(), 1);
      assert.equal(await page.locator('nav [aria-current="page"]').getAttribute('href'), file);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${file} overflows at ${width}`);
      const ids = await page.locator('[id]').evaluateAll(nodes => nodes.map(n => n.id));
      assert.equal(new Set(ids).size, ids.length, 'Duplicate IDs');
      for (const img of await page.locator('img').all()) {
        assert.notEqual(await img.getAttribute('alt'), null, 'Missing image alternative');
        assert.equal(await img.evaluate(n => n.complete && n.naturalWidth > 0), true);
      }
      const refs = await page.locator('a[href],link[href],script[src],img[src]').evaluateAll(nodes => nodes.map(n => ({ value:n.getAttribute(n.hasAttribute('href')?'href':'src'), tag:n.tagName })));
      for (const { value, tag } of refs) {
        assert.ok(value && !value.startsWith('/'), `Root-relative or empty URL: ${value}`);
        const url = new URL(value, page.url());
        if (url.origin === origin) {
          assert.ok(url.pathname.startsWith('/Navzeroth/'), `Escaped project base: ${value}`);
          const response = await context.request.get(url.href);
          assert.equal(response.status(), 200, `Broken local link: ${value}`);
          if (url.hash) {
            const body = await response.text();
            assert.ok(body.includes(`id="${decodeURIComponent(url.hash.slice(1))}"`), `Missing fragment: ${value}`);
          }
        } else {
          assert.equal(tag, 'A', 'Remote runtime asset');
          assert.equal(url.protocol, 'https:');
          if (!draft && url.pathname.includes('/releases/')) {
            assert.ok(release.assets.some(a => a.url === url.href) || url.href === `https://github.com/Laymer/Navzeroth/releases/tag/${release.tag}`, `Unverified release link: ${value}`);
          }
        }
      }
      // The skip link is first, visible on focus, and transfers focus into main.
      await page.keyboard.press('Tab');
      assert.equal(await page.locator(':focus').getAttribute('href'), '#main');
      assert.ok(await page.locator(':focus').evaluate(n => n.getBoundingClientRect().top >= 0));
      await page.keyboard.press('Enter');
      assert.equal(await page.locator(':focus').getAttribute('id'), 'main');
      // Every visible control is reachable without a pointer and has an accessible name.
      await page.goto(`${origin}/Navzeroth/${file}`);
      const controls = await page.locator('a[href],button,summary,[tabindex="0"]').count();
      const reached = new Set();
      for (let index = 0; index < controls; index++) {
        await page.keyboard.press('Tab');
        const focused = await page.locator(':focus').evaluate(n => ({ key:n.outerHTML, name:n.getAttribute('aria-label') || n.textContent.trim(), outline:getComputedStyle(n).outlineStyle }));
        assert.ok(focused.name.length > 0, 'Unnamed keyboard control');
        assert.notEqual(focused.outline, 'none', 'Missing visible focus indication');
        reached.add(focused.key);
      }
      assert.equal(reached.size, controls, `Keyboard reachability: ${file} at ${width}`);
      if (process.env.SITE_SCREENSHOT_DIR && width === 1440) {
        fs.mkdirSync(process.env.SITE_SCREENSHOT_DIR, { recursive:true });
        await page.locator(':focus').evaluate(n => n.blur());
        await page.evaluate(() => scrollTo(0, 0));
        await page.screenshot({ path:path.join(process.env.SITE_SCREENSHOT_DIR, file.replace('.html','.png')), fullPage:true });
      }
    }
  }
  await page.goto(`${origin}/Navzeroth/installation.html`);
  await page.getByRole('button', { name:'Copy checksum command' }).click();
  await page.getByRole('status').filter({ hasText:'Checksum command copied.' }).waitFor();
  assert.equal(await page.evaluate(() => navigator.clipboard.readText()), await page.locator('#checksum-command').textContent());
  assert.deepEqual(errors, []);
  // No script is required to read content or follow the primary navigation.
  const noJs = await browser.newContext({ javaScriptEnabled:false });
  const plain = await noJs.newPage();
  await plain.goto(`${origin}/Navzeroth/index.html`);
  await plain.getByRole('navigation').getByRole('link', { name:'Installation', exact:true }).click();
  assert.ok(plain.url().endsWith('/installation.html'));
  console.log(`Pages validation passed: 5 pages × 3 widths; links, assets, keyboard, semantics, clipboard and no-JS navigation. ${draft ? 'DRAFT: public release/capture verification deliberately not claimed.' : 'Verified release and capture provenance required.'}`);
} finally {
  if (browser) await browser.close();
  await new Promise(resolve => server.close(resolve));
}
