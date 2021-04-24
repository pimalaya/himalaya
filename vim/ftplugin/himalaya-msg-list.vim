setlocal bufhidden=wipe
setlocal buftype=nofile
setlocal cursorline
setlocal nomodifiable
setlocal nowrap

call himalaya#shared#bindings#define([
  \["n", "gm"  , "mbox#input"     ],
  \["n", "gp"  , "mbox#prev_page" ],
  \["n", "gn"  , "mbox#next_page" ],
  \["n", "<cr>", "msg#read"       ],
  \["n", "gw"  , "msg#write"      ],
  \["n", "gr"  , "msg#reply"      ],
  \["n", "gR"  , "msg#reply_all"  ],
  \["n", "gf"  , "msg#forward"    ],
  \["n", "ga"  , "msg#attachments"],
\])
