---
kind: inconsistency
---

# `status` shows a gone upstream as unknown

`ff` reports "upstream gone" when the configured upstream ref no longer
exists, but `status` shows the same state as `?`, which the legend calls
"unknown". `--unsynced` also hides these repositories, although a deleted
upstream branch is arguably what that filter should surface.
