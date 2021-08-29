setlocal bufhidden=wipe
setlocal buftype=nofile
setlocal filetype=mail
setlocal foldexpr=himalaya#shared#thread#fold(v:lnum)
setlocal foldmethod=expr
setlocal nomodifiable

call himalaya#shared#bindings#define([
  \["n", "gw", "msg#write"      ],
  \["n", "gr", "msg#reply"      ],
  \["n", "gR", "msg#reply_all"  ],
  \["n", "gf", "msg#forward"    ],
  \["n", "ga", "msg#attachments"],
  \["n", "gC", "msg#copy"       ],
  \["n", "gM", "msg#move"       ],
  \["n", "gD", "msg#delete"     ],
\])
