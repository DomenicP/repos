---
kind: improvement
---

# The fast-forward progress line uses a fake remote name

`run_fast_forward` reports its last step as `[upstream]: fast-forwarding`,
using the `[remote]` slot of the fetch progress format for something that is
not a remote. Give `FetchProgress` a way to show a plain status message.
