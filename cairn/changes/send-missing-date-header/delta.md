# Delta

## ADDED Requirements

The shared send path SHALL ensure the message carries an origination
date before handing it to any backend: `EmailClient::send_message`
prepends `Date: <now>` (RFC 5322 date-time with the local UTC offset)
when the message has no `Date:` header, and leaves a message that
already carries one byte-identical. This covers every shared command
that sends (`message send`, `compose`, `reply`, `forward`) across
every send-capable backend (SMTP, JMAP, Gmail, Graph). The `smtp send`
plumbing command stays byte-exact and is exempt.
