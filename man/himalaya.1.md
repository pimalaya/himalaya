% himalaya(1) Version 0.6.1 | CLI email management 

NAME
====

**himalaya** — Command-line interface for email management.

SYNOPSIS
========

| **himalaya** \[OPTIONS] \[SUBCOMMAND]

DESCRIPTION
===========

Command-line interface for email management.

Options
-------

-h, \--help

:   Prints brief usage information.

-a, \--account _string_

:   Selects a specific account.

-c, \--config _path_

:   Forces a specific config file path.

-l, \--log-level _level_

:   Defines the logs level *default: info*.

    Possible values: *error*, *warn*, *info*, *debug*, *trace*.

-o, \--output _fmt_

:   Defines the output format *default: plain*
    
    Possible values: *plain*, *json*

-f, \--folder _source_

:   Specifies the source folder *default: inbox*

-v, \--version

:   Prints the current version number.

SUBCOMMANDS
=====

*accounts*

:   Lists accounts

*attachments*

:   Downloads all attachments of the targeted email

*completion*

:   Generates the completion script for the given shell

*copy*

:   Copies an email to the targeted folder

*delete*

:   Deletes an email

*flag*
:   Handles email flags

*folders*

:   Lists folders

*forward*

:   Forwards an email

*help*

:   Prints this message or the help of the given subcommand(s)

*list*

:   Lists all emails

*move*

:   Moves an email to the targeted folder

*notify*

:   Notifies when new messages arrive in the given folder

*read*

:   Reads text bodies of an email

*reply*

:   Answers to an email

*save*

:   Saves a raw email

*search*

:   Lists emails matching the given query

*send*

:   Sends a raw email

*sort*

:   Sorts emails by the given criteria and matching the given query

*template*

:   Handles email templates

*watch*

:   Watches IMAP server changes

*write*

:   Writes a new email


CONFIGURATION
===========

The configuration file is located at: **~/.config/himalaya/config.toml**.
An example might look like:

```
display-name = "Test"
downloads-dir = "~/downloads"
signature = "Regards,"

[gmail]
default = true
email = "test@gmail.com"

backend = "imap" # imap, maildir or notmuch
imap-host = "imap.gmail.com"
imap-port = 993
imap-login = "test@gmail.com"
imap-passwd-cmd = "pass show gmail"

sender = "smtp" # smtp or sendmail
smtp-host = "smtp.gmail.com"
smtp-port = 465
smtp-login = "test@gmail.com"
smtp-passwd-cmd = "security find-internet-password -gs gmail -w"
```

BUGS
====

See GitHub Issues: <https://github.com/soywod/himalaya/issues>

AUTHOR
======

himalaya is written by Clément Douin <clement.douin@posteo.net>.

Manpage was contributed by Michael Vetter <jubalh@iodoru.org>.
