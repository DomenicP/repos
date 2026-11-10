---
kind: improvement
---

# Use more threads for network commands

`workspace::run` caps workers at the CPU count. `fetch` and `ff` mostly wait on
the network and would benefit from more workers.
