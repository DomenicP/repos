---
kind: bug
---

# Fast-forwarding a parent leaves its submodules behind

`checkout_tree` does not update submodules. If the upstream moved a submodule
pointer, the parent shows `subC` right after a successful `ff`, and the next
`ff` refuses it as dirty. Either update submodules to the new pins or report
that they are out of date.
