setlocal cursorline
setlocal foldexpr=himalaya#shared#thread_fold(v:lnum)
setlocal foldlevel=0
setlocal foldlevelstart=0
setlocal foldmethod=expr
setlocal nowrap
setlocal startofline

augroup himalaya
  autocmd! * <buffer>
  autocmd  BufWriteCmd <buffer> call himalaya#msg#draft_save()
  autocmd  BufUnload   <buffer> call himalaya#msg#draft_handle()
augroup end
