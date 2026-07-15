#!/usr/bin/env python3
# The single source of truth for the AESOPUS 1.0 POST body, shared by the fetch tool (aesopus_pull.py) and the
# dry-run provenance battery (aesopus_provenance_test.py). Keeping it here means a refactor of the query semantics
# is caught by the battery's byte-equality check against every vendored provenance record, without touching the
# service. Pure and deterministic: form defaults in, the exact multipart field dict out.
import re


def build_fields(form_html, solmix, zeta_ref, xhmin, fco1, fc1, fn1):
    """Reconstruct the exact multipart POST body from the form defaults and the composition query.

    The form's text and hidden inputs supply the defaults (the T/R grid, the abundance factors, the service dirs);
    the composition query overrides the solar mixture, metallicity, hydrogen abundance (a single X grid point, so
    xhmax = xhmin), and the optional C/O ladder. A "-" C/O argument is the empty string (no ladder)."""
    co = lambda v: "" if v == "-" else v
    fields = {}
    for tag in re.findall(r"<input\b[^>]*>", form_html, flags=re.I):
        nm = re.search(r'name="([^"]*)"', tag)
        if not nm:
            continue
        typ = (
            (re.search(r'type="([^"]*)"', tag) or [None, "text"])[1].lower()
            if re.search(r'type="([^"]*)"', tag)
            else "text"
        )
        if typ in ("submit", "radio"):
            continue
        val = re.search(r'value="([^"]*)"', tag)
        fields[nm.group(1)] = val.group(1) if val else ""
    fields.update(
        {
            "solmix": solmix,
            "ifracn": "1",
            "ialpha": "0",
            "zeta_ref": zeta_ref,
            "xhmin": xhmin,
            "xhmax": xhmin,
            "fco1": co(fco1),
            "fc1": co(fc1),
            "fn1": co(fn1),
            "submit_form": "Submit",
        }
    )
    return fields
