let s:log = function("himalaya#shared#log#info")
let s:trim = function("himalaya#shared#utils#trim")
let s:cli = function("himalaya#shared#cli#call")
let s:plain_req = function("himalaya#request#plain")

let s:msg_id = ""
let s:draft = ""
let s:attachment_paths = []

function! himalaya#msg#list_with(account, mbox, page, should_throw)
  let pos = getpos(".")
  let msgs = s:plain_req({
    \'cmd': '--account %s --mailbox %s list --max-width %d --page %d',
    \'args': [shellescape(a:account), shellescape(a:mbox), s:bufwidth(), a:page],
    \'msg': printf("Fetching %s messages", a:mbox),
    \'should_throw': a:should_throw,
  \})
  let buftype = stridx(bufname("%"), "Himalaya messages") == 0 ? "file" : "edit"
  execute printf("silent! %s Himalaya messages [%s] [page %d]", buftype, a:mbox, a:page)
  setlocal modifiable
  silent execute "%d"
  call append(0, split(msgs, '\n'))
  silent execute "$d"
  setlocal filetype=himalaya-msg-list
  let &modified = 0
  execute 0
  call setpos('.', pos)
endfunction

function! himalaya#msg#list(...)
  try
    if a:0 > 0
      call himalaya#account#set(a:1)
    endif
    let account = himalaya#account#curr()
    let mbox = himalaya#mbox#curr_mbox()
    let page = himalaya#mbox#curr_page()
    call himalaya#msg#list_with(account, mbox, page, 1)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#read()
  try
    let pos = getpos(".")
    let s:msg_id = s:get_focused_msg_id()
    if empty(s:msg_id) || s:msg_id == "HASH" | return | endif
    let account = himalaya#account#curr()
    let mbox = himalaya#mbox#curr_mbox()
    let msg = s:cli(
      \"--account %s --mailbox %s read %s",
      \[shellescape(account), shellescape(mbox), s:msg_id],
      \printf("Fetching message %s", s:msg_id),
      \1,
    \)
    call s:close_open_buffers('Himalaya read message')
    execute printf("silent! botright new Himalaya read message [%s]", s:msg_id)
    setlocal modifiable
    silent execute "%d"
    call append(0, split(substitute(msg, "\r", "", "g"), "\n"))
    silent execute "$d"
    setlocal filetype=himalaya-msg-read
    let &modified = 0
    execute 0
    call setpos('.', pos)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#write(...)
  try
    let pos = getpos(".")
    let account = himalaya#account#curr()
    let msg = a:0 > 0 ? a:1 : s:cli(
      \"--account %s template new",
      \[shellescape(account)],
      \"Fetching new template",
      \0
    \)
    silent! botright new Himalaya write
    setlocal modifiable
    silent execute "%d"
    call append(0, split(substitute(msg, "\r", "", "g"), "\n"))
    silent execute "$d"
    setlocal filetype=himalaya-msg-write
    let &modified = 0
    execute 0
    call setpos('.', pos)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#reply()
  try
    let pos = getpos(".")
    let account = himalaya#account#curr()
    let mbox = himalaya#mbox#curr_mbox()
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id
    let msg = s:cli(
      \"--account %s --mailbox %s template reply %s",
      \[shellescape(account), shellescape(mbox), msg_id],
      \"Fetching reply template",
      \0,
    \)
    execute printf("silent! edit Himalaya reply [%s]", msg_id)
    call append(0, split(substitute(msg, "\r", "", "g"), "\n"))
    silent execute "$d"
    setlocal filetype=himalaya-msg-write
    let &modified = 0
    execute 0
    call setpos('.', pos)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#reply_all()
  try
    let pos = getpos(".")
    let account = himalaya#account#curr()
    let mbox = himalaya#mbox#curr_mbox()
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id
    let msg = s:cli(
      \"--account %s --mailbox %s template reply %s --all",
      \[shellescape(account), shellescape(mbox), msg_id],
      \"Fetching reply all template",
      \0
    \)
    execute printf("silent! edit Himalaya reply all [%s]", msg_id)
    call append(0, split(substitute(msg, "\r", "", "g"), "\n"))
    silent execute "$d"
    setlocal filetype=himalaya-msg-write
    let &modified = 0
    execute 0
    call setpos('.', pos)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#forward()
  try
    let pos = getpos(".")
    let account = himalaya#account#curr()
    let mbox = himalaya#mbox#curr_mbox()
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id
    let msg = s:cli(
      \"--account %s --mailbox %s template forward %s",
      \[shellescape(account), shellescape(mbox), msg_id],
      \"Fetching forward template",
      \0
    \)
    execute printf("silent! edit Himalaya forward [%s]", msg_id)
    call append(0, split(substitute(msg, "\r", "", "g"), "\n"))
    silent execute "$d"
    setlocal filetype=himalaya-msg-write
    let &modified = 0
    execute 0
    call setpos('.', pos)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#copy()
  call himalaya#mbox#pick("himalaya#msg#_copy")
endfunction

function! himalaya#msg#_copy(target_mbox)
  try
    let pos = getpos(".")
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id
    let account = himalaya#account#curr()
    let source_mbox = himalaya#mbox#curr_mbox()
    let msg = s:cli(
      \"--account %s --mailbox %s copy %s %s",
      \[shellescape(account), shellescape(source_mbox), msg_id, shellescape(a:target_mbox)],
      \"Copying message",
      \1,
    \)
    call himalaya#msg#list_with(account, source_mbox, himalaya#mbox#curr_page(), 1)
    call setpos('.', pos)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#move()
  call himalaya#mbox#pick("himalaya#msg#_move")
endfunction

function! himalaya#msg#_move(target_mbox)
  try
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id
    let choice = input(printf("Are you sure you want to move the message %s? (y/N) ", msg_id))
    redraw | echo
    if choice != "y" | return | endif
    let pos = getpos(".")
    let account = himalaya#account#curr()
    let source_mbox = himalaya#mbox#curr_mbox()
    let msg = s:cli(
      \"--account %s --mailbox %s move %s %s",
      \[shellescape(account), shellescape(source_mbox), msg_id, shellescape(a:target_mbox)],
      \"Moving message",
      \1,
    \)
    call himalaya#msg#list_with(account, source_mbox, himalaya#mbox#curr_page(), 1)
    call setpos('.', pos)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#delete() range
  try
    let msg_ids = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_ids(a:firstline, a:lastline) : s:msg_id
    let choice = input(printf("Are you sure you want to delete message(s) %s? (y/N) ", msg_ids))
    redraw | echo
    if choice != "y" | return | endif
    let pos = getpos(".")
    let account = himalaya#account#curr()
    let mbox = himalaya#mbox#curr_mbox()
    let msg = s:cli(
      \"--account %s --mailbox %s delete %s",
      \[shellescape(account), shellescape(mbox), msg_ids],
      \"Deleting message(s)",
      \1,
    \)
    call himalaya#msg#list_with(account, mbox, himalaya#mbox#curr_page(), 1)
    call setpos('.', pos)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#draft_save()
  let s:draft = join(getline(1, "$"), "\n") . "\n"
  redraw | call s:log("Save draft [OK]")
  let &modified = 0
endfunction

function! himalaya#msg#draft_handle()
  try
    let account = himalaya#account#curr()
    let attachments = join(map(s:attachment_paths, "'--attachment '.v:val"), " ")
    while 1
      let choice = input("(s)end, (d)raft, (q)uit or (c)ancel? ")
      let choice = tolower(choice)[0]
      redraw | echo

      if choice == "s"
        return s:cli(
          \"--account %s template send %s -- %s",
          \[shellescape(account), attachments, shellescape(s:draft)],
          \"Sending message",
          \0,
        \)
      elseif choice == "d"
        return s:cli(
          \"--account %s --mailbox Drafts template save %s -- %s",
          \[shellescape(account), attachments, shellescape(s:draft)],
          \"Saving draft",
          \0,
        \)
      elseif choice == "q"
        return
      elseif choice == "c"
        call himalaya#msg#write(join(getline(1, "$"), "\n") . "\n")
        return
      endif
    endwhile
  catch
    call himalaya#shared#log#err(v:exception)
    throw ""
  endtry
endfunction

function! himalaya#msg#attachments()
  try
    let account = himalaya#account#curr()
    let mbox = himalaya#mbox#curr_mbox()
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id
    let msg = s:cli(
      \"--account %s --mailbox %s attachments %s",
      \[shellescape(account), shellescape(mbox), msg_id],
      \"Downloading attachments",
      \0
    \)
    call himalaya#shared#log#info(msg)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#complete_contact(findstart, base)
  try
    if a:findstart
      if !exists("g:himalaya_complete_contact_cmd")
        echoerr "You must set 'g:himalaya_complete_contact_cmd' to complete contacts"
        return -3
      endif

      " search for everything up to the last colon or comma
      let line_to_cursor = getline(".")[:col(".") - 1]
      let start = match(line_to_cursor, '[^:,]*$')

      " don't include leading spaces
      while start <= len(line_to_cursor) && line_to_cursor[start] == " "
        let start += 1
      endwhile

      return start
    else
      let output = system(substitute(g:himalaya_complete_contact_cmd, "%s", a:base, ""))
      let lines = split(output, "\n")

      return map(lines, "s:line_to_complete_item(v:val)")
    endif
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#msg#add_attachment()
  try
    let attachment_path = input("Attachment path: ", "", "file")
    if empty(expand(glob(attachment_path)))
      throw "The file does not exist"
    endif
    call add(s:attachment_paths, attachment_path)
    redraw | call himalaya#shared#log#info("Attachment added!")
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

" Utils

" https://newbedev.com/get-usable-window-width-in-vim-script
function! s:bufwidth()
  let width = winwidth(0)
  let numberwidth = max([&numberwidth, strlen(line('$'))+1])
  let numwidth = (&number || &relativenumber)? numberwidth : 0
  let foldwidth = &foldcolumn

  if &signcolumn == 'yes'
    let signwidth = 2
  elseif &signcolumn == 'auto'
    let signs = execute(printf('sign place buffer=%d', bufnr('')))
    let signs = split(signs, '\n')
    let signwidth = len(signs)>2? 2: 0
  else
    let signwidth = 0
  endif
  return width - numwidth - foldwidth - signwidth
endfunction

function! s:get_msg_id(line)
  return matchstr(a:line, '[0-9a-zA-Z]*')
endfunction

function! s:get_focused_msg_id()
  try
    return s:get_msg_id(getline("."))
  catch
    throw "message not found"
  endtry
endfunction

function! s:get_focused_msg_ids(from, to)
  try
    return join(map(range(a:from, a:to), "s:get_msg_id(getline(v:val))"), ",")
  catch
    throw "messages not found"
  endtry
endfunction

function! s:close_open_buffers(name)
  let open_buffers = filter(range(1, bufnr('$')), 'bufexists(v:val)')
  let target_buffers = filter(open_buffers, 'buffer_name(v:val) =~ a:name')
  for buffer_to_close in target_buffers
    execute ":bwipeout " . buffer_to_close
  endfor
endfunction

function! s:line_to_complete_item(line)
  let fields = split(a:line, "\t")
  let email = fields[0]
  let name = ""
  if len(fields) > 1
    let name = '"' . fields[1] . '" '
  endif

  return name . "<" . email . ">"
endfunction
