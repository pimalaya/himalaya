let s:log = function("himalaya#shared#log#info")
let s:trim = function("himalaya#shared#utils#trim")
let s:cli = function("himalaya#shared#cli#call")

let s:msg_id = 0
let s:draft = ""

" Message

function! s:format_msg_for_list(msg)
  let msg = copy(a:msg)
  let flag_new = index(msg.flags, "Seen") == -1 ? "✷" : " "
  let flag_flagged = index(msg.flags, "Flagged") == -1 ? " " : "!"
  let flag_replied = index(msg.flags, "Answered") == -1 ? " " : "↵"
  let msg.flags = printf("%s %s %s", flag_new, flag_replied, flag_flagged)
  return msg
endfunction

function! himalaya#msg#list_with(account, mbox, page, should_throw)
  let pos = getpos(".")
  let msgs = s:cli(
    \"--account %s --mailbox %s list --page %d",
    \[shellescape(a:account), shellescape(a:mbox), a:page],
    \printf("Fetching %s messages", a:mbox),
    \a:should_throw,
  \)
  let msgs = map(msgs, "s:format_msg_for_list(v:val)")
  let buftype = stridx(bufname("%"), "Himalaya messages") == 0 ? "file" : "edit"
  execute printf("silent! %s Himalaya messages [%s] [page %d]", buftype, a:mbox, a:page + 1)
  setlocal modifiable
  silent execute "%d"
  call append(0, s:render("list", msgs))
  silent execute "$d"
  setlocal filetype=himalaya-msg-list
  let &modified = 0
  execute 0
  call setpos('.', pos)
endfunction

function! himalaya#msg#list(...)
  try
    call himalaya#account#set(a:0 > 0 ? a:1 : "")
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
    let account = himalaya#account#curr()
    let mbox = himalaya#mbox#curr_mbox()
    let msg = s:cli(
      \"--account %s --mailbox %s read %d",
      \[shellescape(account), shellescape(mbox), s:msg_id],
      \printf("Fetching message %d", s:msg_id),
      \0,
    \)
    let attachment = msg.hasAttachment ? " []" : ""
    execute printf("silent! edit Himalaya read message [%d]%s", s:msg_id, attachment)
    setlocal modifiable
    silent execute "%d"
    call append(0, split(substitute(msg.content, "\r", "", "g"), "\n"))
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

function! himalaya#msg#write()
  try
    let pos = getpos(".")
    let account = himalaya#account#curr()
    let msg = s:cli("--account %s template new", [shellescape(account)], "Fetching new template", 0)
    silent! edit Himalaya write
    call append(0, split(substitute(msg.raw, "\r", "", "g"), "\n"))
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
      \"--account %s --mailbox %s template reply %d",
      \[shellescape(account), shellescape(mbox), msg_id],
      \"Fetching reply template",
      \0,
    \)
    execute printf("silent! edit Himalaya reply [%d]", msg_id)
    call append(0, split(substitute(msg.raw, "\r", "", "g"), "\n"))
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
      \"--account %s --mailbox %s template reply %d --all",
      \[shellescape(account), shellescape(mbox), msg_id],
      \"Fetching reply all template",
      \0
    \)
    execute printf("silent! edit Himalaya reply all [%d]", msg_id)
    call append(0, split(substitute(msg.raw, "\r", "", "g"), "\n"))
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
      \"--account %s --mailbox %s template forward %d",
      \[shellescape(account), shellescape(mbox), msg_id],
      \"Fetching forward template",
      \0
    \)
    execute printf("silent! edit Himalaya forward [%d]", msg_id)
    call append(0, split(substitute(msg.raw, "\r", "", "g"), "\n"))
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
      \"--account %s --mailbox %s copy %d %s",
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
    let choice = input(printf("Are you sure you want to move the message %d? (y/N) ", msg_id))
    redraw | echo
    if choice != "y" | return | endif
    let pos = getpos(".")
    let account = himalaya#account#curr()
    let source_mbox = himalaya#mbox#curr_mbox()
    let msg = s:cli(
      \"--account %s --mailbox %s move %d %s",
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
  let s:draft = join(getline(1, "$"), "\n")
  redraw | call s:log("Save draft [OK]")
  let &modified = 0
endfunction

function! himalaya#msg#draft_handle()
  try
    let account = himalaya#account#curr()
    while 1
      let choice = input("(s)end, (d)raft, (q)uit or (c)ancel? ")
      let choice = tolower(choice)[0]
      redraw | echo

      if choice == "s"
        return s:cli(
          \"--account %s send -- %s",
          \[shellescape(account), shellescape(s:draft)],
          \"Sending message",
          \0,
        \)
      elseif choice == "d"
        return s:cli(
          \"--account %s --mailbox Drafts save -- %s",
          \[shellescape(account), shellescape(s:draft)],
          \"Saving draft",
          \0,
        \)
      elseif choice == "q"
        return
      elseif choice == "c"
        throw "Action canceled"
      endif
    endwhile
  catch
    " TODO: find a better way to prevent the buffer to close (stop the BufUnload event)
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
      \"--account %s --mailbox %s attachments %d",
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

" Render utils

let s:config = {
  \"list": {
    \"columns": ["uid", "flags", "subject", "sender", "date"],
  \},
  \"labels": {
    \"uid": "UID",
    \"flags": "FLAGS",
    \"subject": "SUBJECT",
    \"sender": "SENDER",
    \"date": "DATE",
  \},
\}

function! s:render(type, lines)
  let s:max_widths = s:get_max_widths(a:lines, s:config[a:type].columns)
  let header = [s:render_line(s:config.labels, s:max_widths, a:type)]
  let line = map(copy(a:lines), "s:render_line(v:val, s:max_widths, a:type)")

  return header + line
endfunction

function! s:render_line(line, max_widths, type)
  return "|" . join(map(
    \copy(s:config[a:type].columns),
    \"s:render_cell(a:line[v:val], a:max_widths[v:key])",
  \), "")
endfunction

function! s:render_cell(cell, max_width)
  let cell_width = strdisplaywidth(a:cell[:a:max_width])
  return a:cell[:a:max_width] . repeat(" ", a:max_width - cell_width) . " |"
endfunction

function! s:get_max_widths(msgs, columns)
  let max_widths = map(copy(a:columns), "strlen(s:config.labels[v:val])")

  for msg in a:msgs
    let widths = map(copy(a:columns), "has_key(msg, v:val . '_len') ? msg[v:val . '_len'] : strlen(msg[v:val])")
    call map(max_widths, "max([widths[v:key], v:val])")
  endfor

  return max_widths
endfunction

function! s:get_focused_msg_id()
  try
    return s:trim(split(getline("."), "|")[0])
  catch
    throw "message not found"
  endtry
endfunction

function! s:get_focused_msg_ids(from, to)
  try
    return join(map(range(a:from, a:to), "s:trim(split(getline(v:val), '|')[0])"), ",")
  catch
    throw "messages not found"
  endtry
endfunction
