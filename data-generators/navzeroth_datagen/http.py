"""Cached, rate-limited HTTPS fetches shared by every generator.

Downloads land in data-generators/.cache/<host>/ (git-ignored) with a .meta.json
beside each body (url, status, sha256, bytes, retrieved_at_utc_ms). A failed
status (403, 404) is cached too, so an --offline rebuild reproduces the same
fallbacks. Requests to one host share a cross-process limit (concurrent slots
and a minimum interval) through lock files in .cache/locks/, so generators run
in parallel by run_all.py stay polite together. Redirects to another host are
refused. Tests never use the network: set NAVZEROTH_DATAGEN_OFFLINE=1.

    from navzeroth_datagen import http
    text = http.fetch_text("https://wago.tools/db2/AreaTable/csv?product=wowf")
"""
import contextlib
import hashlib
import json
import os
import pathlib
import re
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

try:
    import fcntl
except ImportError:  # Windows: limits hold per process only
    fcntl = None

ROOT = pathlib.Path(__file__).resolve().parent.parent
USER_AGENT = "Mozilla/5.0 (Navzeroth data generator; https://github.com/Laymer/Navzeroth)"
# host -> (concurrent requests across all processes, minimum seconds between starts)
HOST_LIMITS = {
    # Intervals match the source registry's minimum_interval_ms (1000) for these sources.
    "www.wowhead.com": (2, 1.0),
    "nether.wowhead.com": (2, 1.0),
    "wago.tools": (2, 1.0),
    "api.github.com": (1, 1.0),
    "github.com": (1, 1.0),
    "raw.githubusercontent.com": (1, 1.0),
}
DEFAULT_LIMIT = (1, 1.0)
RETRY_STATUS = {429, 500, 502, 503, 504}
# Answers that may be transient (rate limits, bot challenges): used for this run, never cached.
NOT_CACHED = RETRY_STATUS | {202, 403}


class FetchError(Exception):
    def __init__(self, url, status, message=""):
        super().__init__(f"{status} {url} {message}".strip())
        self.url, self.status = url, status


def cache_root():
    return pathlib.Path(os.environ.get("NAVZEROTH_DATAGEN_CACHE") or ROOT / ".cache")


def offline():
    return os.environ.get("NAVZEROTH_DATAGEN_OFFLINE") == "1"


def refresh():
    return os.environ.get("NAVZEROTH_DATAGEN_REFRESH") == "1"


def configure(*, refresh_cache=None, offline_only=None, cache=None):
    """Set the process-wide mode; run_all.py passes it to workers through the env."""
    if refresh_cache is not None:
        os.environ["NAVZEROTH_DATAGEN_REFRESH"] = "1" if refresh_cache else "0"
    if offline_only is not None:
        os.environ["NAVZEROTH_DATAGEN_OFFLINE"] = "1" if offline_only else "0"
    if cache is not None:
        os.environ["NAVZEROTH_DATAGEN_CACHE"] = str(cache)


_used = {}
_used_lock = threading.Lock()
_inflight = {}
_inflight_lock = threading.Lock()


def used():
    """Every cache entry this process read: {url: meta}, for source digests."""
    with _used_lock:
        return dict(_used)


def snapshot_digest(prefix):
    """SHA-256 over the sorted (url, sha256) pairs used whose URL starts with prefix."""
    pairs = sorted((u, m.get("sha256") or str(m.get("status"))) for u, m in used().items() if u.startswith(prefix))
    return hashlib.sha256(json.dumps(pairs).encode()).hexdigest()


def _paths(url, name):
    host = urllib.parse.urlsplit(url).hostname or "unknown"
    key = re.sub(r"[^A-Za-z0-9._-]", "_", os.path.basename(name or "")) if name else ""
    key = key.lstrip(".") or hashlib.sha256(url.encode()).hexdigest()
    base = cache_root() / host / key
    return base, base.with_name(base.name + ".meta.json")


@contextlib.contextmanager
def _flock(path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a+") as f:
        if fcntl:
            fcntl.flock(f, fcntl.LOCK_EX)
        try:
            yield f
        finally:
            if fcntl:
                fcntl.flock(f, fcntl.LOCK_UN)


_thread_slots = {}


@contextlib.contextmanager
def _host_slot(host):
    """Hold one of the host's concurrency slots and respect its interval."""
    slots, interval = HOST_LIMITS.get(host, DEFAULT_LIMIT)
    with _inflight_lock:
        sem = _thread_slots.setdefault(host, threading.BoundedSemaphore(slots))
    locks = cache_root() / "locks"
    locks.mkdir(parents=True, exist_ok=True)
    with sem:
        held = None
        while held is None:
            for i in range(slots):
                f = open(locks / f"{host}.{i}.lock", "a+")
                try:
                    if fcntl:
                        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    held = f
                    break
                except OSError:
                    f.close()
            if held is None:
                time.sleep(0.05)
        try:
            with _flock(locks / f"{host}.last") as stamp:
                stamp.seek(0)
                last = float(stamp.read() or 0)
                wait = last + interval - time.time()
                if wait > 0:
                    time.sleep(wait)
                stamp.seek(0)
                stamp.truncate()
                stamp.write(repr(time.time()))
            yield
        finally:
            if fcntl:
                fcntl.flock(held, fcntl.LOCK_UN)
            held.close()


class _SameHostRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        new, old = urllib.parse.urlsplit(newurl), urllib.parse.urlsplit(req.full_url)
        if new.hostname != old.hostname or new.scheme != "https":
            raise FetchError(req.full_url, code, f"redirect to another host refused: {newurl}")
        return super().redirect_request(req, fp, code, msg, headers, newurl)


_opener = urllib.request.build_opener(_SameHostRedirects)


def _download(url, timeout):
    host = urllib.parse.urlsplit(url).hostname
    if urllib.parse.urlsplit(url).scheme != "https":
        raise FetchError(url, 0, "only https is fetched")
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(4):
        try:
            with _host_slot(host):
                with _opener.open(req, timeout=timeout) as r:
                    return r.status, r.read()
        except urllib.error.HTTPError as e:
            if e.code in RETRY_STATUS and attempt < 3:
                time.sleep(2 ** attempt)
                continue
            return e.code, None
        except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
            if attempt == 3:
                raise FetchError(url, 0, str(e)) from e
            time.sleep(2 ** attempt)
    raise FetchError(url, 0, "retries exhausted")


def fetch(url, name=None, *, timeout=60):
    """Bytes of url from the cache or the network; raises FetchError on a failed status.
    `name` is an optional readable cache file name (unique per host)."""
    body_path, meta_path = _paths(url, name)
    with _inflight_lock:
        local = _inflight.setdefault(str(body_path), threading.Lock())
    with local, _flock(meta_path.with_name(meta_path.name + ".lock")):
        meta = None
        if meta_path.exists() and not refresh():
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
        elif offline():
            raise FetchError(url, 0, "not cached (offline)")
        else:
            status, data = _download(url, timeout)
            meta = {"url": url, "status": status, "retrieved_at_utc_ms": int(time.time() * 1000)}
            if data is not None and 200 <= status < 300 and status != 202:
                body_path.write_bytes(data)
                meta.update(sha256=hashlib.sha256(data).hexdigest(), bytes=len(data))
            if status not in NOT_CACHED:  # a refresh that hits a 5xx keeps the good entry
                meta_path.write_text(json.dumps(meta, sort_keys=True), encoding="utf-8")
        with _used_lock:
            _used[url] = meta
        if not (200 <= meta["status"] < 300) or meta["status"] == 202:
            raise FetchError(url, meta["status"])
        return body_path.read_bytes()


def fetch_text(url, name=None, *, timeout=60):
    return fetch(url, name, timeout=timeout).decode("utf-8", errors="replace")


def fetch_json(url, name=None, *, timeout=60):
    return json.loads(fetch_text(url, name, timeout=timeout))
