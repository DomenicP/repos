---
kind: improvement
---

# A failing unrelated remote blocks fast-forward

`run_fast_forward` stops if any remote fails to fetch. A dead fork remote or
an extra mirror therefore prevents the fast-forward even when the upstream's
own remote fetched successfully. Only the upstream's remote needs to succeed;
report the other failures without blocking the update.
