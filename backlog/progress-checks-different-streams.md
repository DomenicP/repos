---
kind: inconsistency
---

# The progress displays check different streams

`status_progress` decides whether to show a spinner by checking stdout, but
`FetchProgress` checks stderr. Both draw to stderr.
