---
cairn: log
change: compose-message-id-domain
landed: 2026-10-09
---

# The Message-ID sits on the sender's domain

`message compose`, `reply` and `forward` left the `Message-ID` to mail-builder, which generates it on the hostname of the machine when none is set: a message composed on a machine called `nixos` went out as `<…@nixos>`. That tells every recipient the name of the machine, and a bare hostname is rarely a domain at all, where RFC 5322 section 3.6.4 counts on a domain name on the right-hand side to make the id globally unique.

The builder now sets the header itself from the domain of the `From` address, the left-hand side still coming from mail-builder's own generator. With no `From`, or a domain that is not a plain dot-atom (a domain literal, an internationalized domain), nothing is set and mail-builder falls back to the hostname as before.

Spec unchanged.
