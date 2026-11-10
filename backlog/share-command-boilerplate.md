---
kind: improvement
---

# Share the command boilerplate

`run_fast_forward`, `run_fetch` and `run_status` each resolve the workspace,
discover repositories, print "No repositories found.", report errors and choose
the exit code. A shared reporting helper would remove the duplication and the
small differences (for example, only `run_status` locks stderr once).
