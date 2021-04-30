setlocal cursorline
setlocal filetype=mail
setlocal foldexpr=himalaya#shared#thread#fold(v:lnum)
setlocal foldmethod=expr
setlocal startofline
syntax on

augroup himalaya_write
  autocmd! * <buffer>
  autocmd  BufWriteCmd <buffer> call himalaya#msg#draft_save()
  autocmd  BufUnload   <buffer> call himalaya#msg#draft_handle()
augroup end
