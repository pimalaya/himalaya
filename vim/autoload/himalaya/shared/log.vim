function! himalaya#shared#log#info(msg)
  echohl None
  echomsg a:msg
endfunction

function! himalaya#shared#log#err(msg)
  echohl ErrorMsg
  echomsg a:msg
  echohl None
endfunction
