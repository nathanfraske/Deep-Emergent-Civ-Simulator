#!/usr/bin/env python3
# Vendored ÆSOPUS 1.0 (Marigo & Aringer 2009) gas-only opacity fetcher, the provenance-protocol tool for
# the kappa_R molecular gap-filler. Reproduces a grid from its recorded query: it POSTs the composition to
# https://stev.oapd.inaf.it/cgi-bin/aesopus_1.0, polls the async result, verifies the completion marker,
# and writes the table plus a provenance JSON (full POST body + service banner/date + our md5). The vendored
# .dat is the immutable artifact; the service is a courtesy and is never re-pulled. Requires aesopus_1.0_form.html
# (the fetched input form) in the working directory. TLS verify is off only for the academic CA gap; integrity
# rests on the md5 and the in-file service banner, not the transport.
"""Pull one AESOPUS 1.0 gas-only opacity table under the provenance protocol:
POST the composition, poll the async result, verify the completion marker, record
query + service banner + our md5, and vendor the immutable copy.

Usage: aesopus_pull.py <label> <solmix> <zeta_ref> <xhmin> <fco1> <fc1> <fn1>
  empty C/O ladder args as "-".
"""
import re, sys, time, json, hashlib, urllib.parse, requests

BASE = "https://stev.oapd.inaf.it/cgi-bin"
FORM_URL = BASE + "/aesopus_1.0"
FORM_HTML = "aesopus_1.0_form.html"
POLL_SECS, MAX_WAIT = 15, 420

label, solmix, zeta_ref, xhmin, fco1, fc1, fn1 = sys.argv[1:8]
co = lambda v: "" if v == "-" else v

# Parse the form defaults (text + hidden), then set the requested composition.
form_html = open(FORM_HTML).read()
fields = {}
for tag in re.findall(r"<input\b[^>]*>", form_html, flags=re.I):
    nm = re.search(r'name="([^"]*)"', tag)
    if not nm:
        continue
    typ = (re.search(r'type="([^"]*)"', tag) or [None, "text"])[1].lower() \
          if re.search(r'type="([^"]*)"', tag) else "text"
    if typ in ("submit", "radio"):
        continue
    val = re.search(r'value="([^"]*)"', tag)
    fields[nm.group(1)] = val.group(1) if val else ""
fields.update({"solmix": solmix, "ifracn": "1", "ialpha": "0",
               "zeta_ref": zeta_ref, "xhmin": xhmin, "xhmax": xhmin,
               "fco1": co(fco1), "fc1": co(fc1), "fn1": co(fn1),
               "submit_form": "Submit"})

multipart = {k: (None, v) for k, v in fields.items()}
r = requests.post(FORM_URL, files=multipart, timeout=180, verify=False)
r.raise_for_status()
submitted = (re.search(r'submitted on ([^\n.<]+)', r.text) or [None, "?"])[1].strip()
# The href is like "../tmp/output<id>.dat"; resolve against the endpoint (the <base> makes ../ climb from
# /cgi-bin/ to the server root /tmp/, verified against the live server).
href = re.search(r'href=(\.\./tmp/output\d+\.dat)', r.text) or \
       re.search(r'href=([^ >]*output\d+\.dat)', r.text)
if not href:
    print("NO OUTPUT LINK; response head:\n", r.text[:800]); sys.exit(1)
out_url = urllib.parse.urljoin(FORM_URL, href.group(1))
print("submitted:", submitted, "| out_url:", out_url)

# Poll until the completion marker appears.
waited, content = 0, ""
while waited < MAX_WAIT:
    time.sleep(POLL_SECS); waited += POLL_SECS
    g = requests.get(out_url, timeout=120, verify=False)
    content = g.text
    if "AESOPUS computation completed" in content:
        print("COMPLETED after ~%ds" % waited); break
    print("  waiting (%ds), len=%d" % (waited, len(content)))
else:
    print("TIMEOUT; last len", len(content)); sys.exit(2)

raw = g.content
md5 = hashlib.md5(raw).hexdigest()
import os
outdir = os.environ.get("AESOPUS_OUTDIR", ".")
os.makedirs(outdir, exist_ok=True)
fname = os.path.join(outdir, "aesopus1.0_gasonly_%s.dat" % label)
open(fname, "wb").write(raw)
prov = {"label": label, "service": "AESOPUS 1.0 (Marigo & Aringer 2009)",
        "endpoint": FORM_URL, "submitted_banner": submitted,
        "query": {k: fields[k] for k in ("solmix", "zeta_ref", "xhmin", "fco1", "fc1", "fn1",
                                          "lgtmin", "lgtmax", "dlgt1", "lgtchange", "dlgt2",
                                          "lgrmin", "lgrmax", "dlgr", "ifracn", "ialpha",
                                          "aesopus_version", "alih")},
        "full_post_fields": fields, "our_md5": md5, "bytes": len(raw), "out_url": out_url}
json.dump(prov, open(fname + ".provenance.json", "w"), indent=2)
print("VENDORED", fname, "md5", md5, "bytes", len(raw))
# Show the header so we can build the loader adapter (rider 4).
print("--- output header (first 30 lines) ---")
print("\n".join(content.splitlines()[:30]))
