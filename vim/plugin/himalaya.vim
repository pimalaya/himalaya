if exists("g:loaded_himalaya")
  finish
endif

if !executable("himalaya")
  throw "Himalaya CLI not found, see https://github.com/soywod/himalaya#installation"
endif

" Backup cpo
let s:cpo_backup = &cpo
set cpo&vim

command! -nargs=* Himalaya            call himalaya#msg#list(<f-args>)
command! -nargs=* HimalayaMove        call himalaya#msg#move()
command! -nargs=* HimalayaCopy        call himalaya#msg#copy()
command! -nargs=* HimalayaDelete      call himalaya#msg#delete()
command! -nargs=* HimalayaWrite       call himalaya#msg#write()
command! -nargs=* HimalayaReply       call himalaya#msg#reply()
command! -nargs=* HimalayaReplyAll    call himalaya#msg#reply_all()
command! -nargs=* HimalayaForward     call himalaya#msg#forward()
command! -nargs=1 HimalayaMbox        call himalaya#mbox#_change(<f-args>)
command! -nargs=* HimalayaMboxList    call himalaya#mbox#change()
command! -nargs=* HimalayaNextPage    call himalaya#mbox#next_page()
command! -nargs=* HimalayaPrevPage    call himalaya#mbox#prev_page()
command! -nargs=* HimalayaAttach      call himalaya#msg#add_attachment()
command! -nargs=* HimalayaAttachments call himalaya#msg#attachments()

" Restore cpo
let &cpo = s:cpo_backup
unlet s:cpo_backup

let g:loaded_himalaya = 1
