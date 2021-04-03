let s:print_info = function("himalaya#utils#print_msg")
let s:print_err = function("himalaya#utils#print_err")
let s:trim = function("himalaya#utils#trim")
let s:cli = function("himalaya#shared#cli")

let s:msg_id = 0
let s:draft = ""

" Message

function! s:format_msg_for_list(msg)
  let msg = copy(a:msg)

  let flag_unseen = index(msg.flags, "Seen") == -1 ? "🟓" : " "
  let flag_replied = index(msg.flags, "Answered") == -1 ? " " : "↩"
  let flag_flagged = index(msg.flags, "Flagged") == -1 ? " " : "!"
  let msg.flags = printf("%s%s%s", flag_unseen, flag_replied, flag_flagged)

  return msg
endfunction

function! himalaya#msg#list()
  try
    let mbox = himalaya#mbox#curr_mbox()
    let page = himalaya#mbox#curr_page()

    call s:print_info(printf("Fetching %s messages…", tolower(mbox)))
    let msgs = s:cli("--mailbox %s list --page %d", [shellescape(mbox), page])
    let msgs = map(copy(msgs), "s:format_msg_for_list(v:val)")
    call s:print_info("Done!")

    let buftype = stridx(bufname("%"), "Himalaya messages") == 0 ? "file" : "edit"
    execute printf("silent! %s Himalaya messages [%s] [page %d]", buftype, tolower(mbox), page + 1)
    setlocal modifiable
    execute "%d"
    call append(0, s:render("list", msgs))
    execute "$d"
    setlocal filetype=himalaya-msg-list
    let &modified = 0
    execute 0
  catch
    call s:print_err(v:exception)
  endtry
endfunction

function! himalaya#msg#read()
  try
    let s:msg_id = s:get_focused_msg_id()
    let mbox = himalaya#mbox#curr_mbox()

    call s:print_info(printf("Fetching message %d…", s:msg_id))
    let msg = s:cli("read %d --mailbox %s", [s:msg_id, shellescape(mbox)])
    call s:print_info("Done!")

    let attachment = msg.hasAttachment ? " []" : ""
    execute printf("silent! edit Himalaya read message [%d]%s", s:msg_id, attachment)
    setlocal modifiable
    execute "%d"
    call append(0, split(substitute(msg.content, "\r", "", "g"), "\n"))
    execute "$d"
    setlocal filetype=himalaya-msg-read
    let &modified = 0
    execute 0
  catch
    call s:print_err(v:exception)
  endtry
endfunction

function! himalaya#msg#write()
  try
    call s:print_info("Fetching new template…")
    let msg = s:cli("template new", [])
    call s:print_info("Done!")

    silent! edit Himalaya write
    call append(0, split(substitute(msg.template, "\r", "", "g"), "\n"))
    execute "$d"
    setlocal filetype=himalaya-msg-write
    let &modified = 0
    execute 0
  catch
    call s:print_err(v:exception)
  endtry
endfunction

function! himalaya#msg#reply()
  try
    let mbox = himalaya#mbox#curr_mbox()
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id

    call s:print_info("Fetching reply template…")
    let msg = s:cli("template reply %d --mailbox %s", [msg_id, shellescape(mbox)])
    call s:print_info("Done!")

    execute printf("silent! edit Himalaya reply [%d]", msg_id)
    call append(0, split(substitute(msg.template, "\r", "", "g"), "\n"))
    execute "$d"
    setlocal filetype=himalaya-msg-write
    let &modified = 0
    execute 0
  catch
    call s:print_err(v:exception)
  endtry
endfunction

function! himalaya#msg#reply_all()
  try
    let mbox = himalaya#mbox#curr_mbox()
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id

    call s:print_info("Fetching reply all template…")
    let msg = s:cli("template reply %d --mailbox %s --all", [msg_id, shellescape(mbox)])
    call s:print_info("Done!")

    execute printf("silent! edit Himalaya reply all [%d]", msg_id)
    call append(0, split(substitute(msg.template, "\r", "", "g"), "\n"))
    execute "$d"
    setlocal filetype=himalaya-msg-write
    let &modified = 0
    execute 0
  catch
    call s:print_err(v:exception)
  endtry
endfunction

function! himalaya#msg#forward()
  try
    let mbox = himalaya#mbox#curr_mbox()
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id

    call s:print_info("Fetching forward template…")
    let msg = s:cli("template forward %d --mailbox %s", [msg_id, shellescape(mbox)])
    call s:print_info("Done!")

    execute printf("silent! edit Himalaya forward [%d]", msg_id)
    call append(0, split(substitute(msg.template, "\r", "", "g"), "\n"))
    execute "$d"
    setlocal filetype=himalaya-msg-write
    let &modified = 0
    execute 0
  catch
    call s:print_err(v:exception)
  endtry
endfunction

function! himalaya#msg#draft_save()
  let s:draft = join(getline(1, "$"), "\r\n")
  call s:print_info("Draft saved!")
  let &modified = 0
endfunction

function! himalaya#msg#draft_handle()
  while 1
    let choice = input("(s)end, (d)raft, (q)uit or (c)ancel? ")
    let choice = tolower(choice)[0]
    redraw | echo

    if choice == "s"
      call s:print_info("Sending message…")
      call s:cli("send -- %s", [shellescape(s:draft)])
      call s:print_info("Done!")
      return
    elseif choice == "d"
      call s:print_info("Saving draft…")
      call s:cli("save --mailbox Drafts -- %s", [shellescape(s:draft)])
      call s:print_info("Done!")
      return
    elseif choice == "q"
      return
    elseif choice == "c"
      throw "Action canceled"
    endif
  endwhile
endfunction

function! himalaya#msg#attachments()
  try
    let mbox = himalaya#mbox#curr_mbox()
    let msg_id = stridx(bufname("%"), "Himalaya messages") == 0 ? s:get_focused_msg_id() : s:msg_id

    call s:print_info("Downloading attachments…")
    let msg = s:cli("attachments %d --mailbox %s", [msg_id, shellescape(mbox)])
    call s:print_info("Done!")
  catch
    call s:print_err(v:exception)
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
    let widths = map(copy(a:columns), "strlen(msg[v:val])")
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
