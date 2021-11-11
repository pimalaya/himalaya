setlocal filetype=mail
setlocal foldexpr=himalaya#shared#thread#fold(v:lnum)
setlocal foldmethod=expr
setlocal startofline

if exists("g:himalaya_complete_contact_cmd")
  setlocal completefunc=himalaya#msg#complete_contact
endif

augroup himalaya_write
  autocmd! * <buffer>
  autocmd  BufWriteCmd <buffer> call himalaya#msg#draft_save()
  autocmd  BufLeave    <buffer> call himalaya#msg#draft_handle()
augroup end
