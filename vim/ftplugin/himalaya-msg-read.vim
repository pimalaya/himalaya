setlocal bufhidden=wipe
setlocal buftype=nofile
setlocal cursorline
setlocal foldexpr=himalaya#shared#thread#fold(v:lnum)
setlocal foldlevel=0
setlocal foldlevelstart=0
setlocal foldmethod=expr
setlocal nomodifiable
setlocal nowrap
setlocal startofline

call himalaya#shared#bindings#define([
  \["n", "gw", "msg#write"      ],
  \["n", "gr", "msg#reply"      ],
  \["n", "gR", "msg#reply_all"  ],
  \["n", "gf", "msg#forward"    ],
  \["n", "ga", "msg#attachments"],
\])
