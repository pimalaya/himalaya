function! himalaya#shared#thread#fold(lnum)
  return getline(a:lnum)[0] == ">"
endfunction
