if exists("g:himalaya_loaded")
  finish
endif

let g:himalaya_loaded = 1

if !executable("himalaya")
  throw "Himalaya CLI not found, see https://github.com/soywod/himalaya#installation"
endif

command! -nargs=* Himalaya call himalaya#msg#list(<f-args>)
