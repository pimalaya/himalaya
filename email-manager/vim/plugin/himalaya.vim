if exists("g:loaded_himalaya")
  finish
endif

if !executable("himalaya")
  throw "Himalaya CLI not found, see https://github.com/soywod/himalaya#installation"
endif

" Backup cpo
let s:cpo_backup = &cpo
set cpo&vim

command! -nargs=* Himalaya call himalaya#msg#list(<f-args>)

" Restore cpo
let &cpo = s:cpo_backup
unlet s:cpo_backup

let g:loaded_himalaya = 1
