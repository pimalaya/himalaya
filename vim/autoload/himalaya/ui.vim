let s:compose = function('himalaya#utils#compose')
let s:trim = function('himalaya#utils#trim')
let s:print_msg = function('himalaya#utils#print_msg')
let s:print_err = function('himalaya#utils#print_err')

let s:max_widths = []
let s:buff_name = 'Himalaya'
let s:msgs = []

let s:config = {
  \'list': {
    \'columns': ['uid', 'subject', 'sender', 'date'],
  \},
  \'labels': {
    \'uid': 'ID',
    \'subject': 'SUBJECT',
    \'sender': 'SENDER',
    \'date': 'DATE',
  \},
\}

function! himalaya#ui#list()
  try
    let prev_pos = getpos('.')
    let s:msgs = himalaya#msg#list()
    let lines = map(copy(s:msgs), 'himalaya#msg#format_for_list(v:val)')

    redir => buf_list | silent! ls | redir END
    execute 'silent! edit ' . s:buff_name

    if match(buf_list, '"Himalaya') > -1
      execute '0,$d'
    endif

    call append(0, s:render('list', lines))
    execute '$d'
    call setpos('.', prev_pos)
    setlocal filetype=himalaya-list
    let &modified = 0
    echo
  catch
    call s:print_err(v:exception)
  endtry
endfunction

" Cell management

function! himalaya#ui#select_next_cell()
  normal! f|l

  if col('.') == col('$') - 1
    if line('.') == line('$')
      normal! T|
    else
      normal! j0l
    endif
  endif
endfunction

function! himalaya#ui#select_prev_cell()
  if col('.') == 2 && line('.') > 2
    normal! k$T|
  else
    normal! 2T|
  endif
endfunction

function! himalaya#ui#delete_in_cell()
  execute printf('normal! %sdt|', col('.') == 1 ? '' : 'T|')
endfunction

function! himalaya#ui#change_in_cell()
  call himalaya#ui#delete_in_cell()
  startinsert
endfunction

function! himalaya#ui#visual_in_cell()
  execute printf('normal! %svt|', col('.') == 1 ? '' : 'T|')
endfunction

" Parse utils

function! himalaya#ui#parse_buffer()
  " try
    " let lines = filter(getline(2, "$"), "!empty(s:trim(v:val))")
    " let prev_msgs = copy(s:msgs)
    " let next_msgs = map(lines, "s:parse_buffer_line(v:key, v:val)")
    " let msgs_to_add = filter(copy(next_msgs), "empty(v:val.id)")
    " let msgs_to_edit = []
    " let msgs_to_do = []
    " let msgs = []

    " for prev_msg in prev_msgs
    "   let next_msg = filter(copy(next_msgs), "v:val.id == prev_msg.id")

    "   if empty(next_msg)
    "     let msgs_to_do += [prev_msg.id]
    "   elseif prev_msg.desc != next_msg[0].desc || prev_msg.project != next_msg[0].project || prev_msg.due.approx != next_msg[0].due
    "     let msgs_to_edit += [next_msg[0]]
    "   endif
    " endfor

    " for msg in msgs_to_add  | let msgs += [himalaya#msg#add(msg)]  | endfor
    " for msg in msgs_to_edit | let msgs += [himalaya#msg#edit(msg)] | endfor
    " for id in msgs_to_do     | let msgs += [himalaya#msg#do(id)]     | endfor 

    " call himalaya#ui#list()
    " let &modified = 0
    " for msg in msgs | call s:print_msg(msg) | endfor
  " catch
  "   call s:print_err(v:exception)
  " endtry
endfunction

function! s:parse_buffer_line(index, line)
  if match(a:line, '^|[0-9a-f\-]\{-} *|.* *|.\{-} *|.\{-} *|.\{-} *|$') != -1
    let cells = split(a:line, "|")
    let id = s:trim(cells[0])
    let desc = s:trim(join(cells[1:-4], ""))
    let project = s:trim(cells[-3])
    let due = s:trim(cells[-1])

    return {
      \"id": id,
      \"desc": desc,
      \"project": project,
      \"due": due,
    \}
  else
    let [desc, project, due] = s:parse_args(s:trim(a:line))

    return {
      \"id": "",
      \"desc": desc,
      \"project": project,
      \"due": due,
    \}
  endif
endfunction

function! s:uniq_by_id(a, b)
  if a:a.id > a:b.id | return 1
  elseif a:a.id < a:b.id | return -1
  else | return 0 | endif
endfunction

function! s:parse_args(args)
  let args = split(a:args, ' ')

  let idx = 0
  let desc = []
  let project = ""
  let due = ""

  while idx < len(args)
    let arg = args[idx]

    if arg == "-p" || arg == "--project"
      let project = get(args, idx + 1, "")
      let idx = idx + 1
    elseif arg == "-d" || arg == "--due"
      let due = get(args, idx + 1, "")
      let idx = idx + 1
    else
      call add(desc, arg)
    endif

    let idx = idx + 1
  endwhile

  return [join(desc, ' '), project, due]
endfunction

" ------------------------------------------------------------------ # Renders #

function! s:render(type, lines)
  let s:max_widths = s:get_max_widths(a:lines, s:config[a:type].columns)
  let header = [s:render_line(s:config.labels, s:max_widths, a:type)]
  let line = map(copy(a:lines), 's:render_line(v:val, s:max_widths, a:type)')

  return header + line
endfunction

function! s:render_line(line, max_widths, type)
  return '|' . join(map(
    \copy(s:config[a:type].columns),
    \'s:render_cell(a:line[v:val], a:max_widths[v:key])',
  \), '')
endfunction

function! s:render_cell(cell, max_width)
  let cell_width = strdisplaywidth(a:cell[:a:max_width])
  return a:cell[:a:max_width] . repeat(' ', a:max_width - cell_width) . ' |'
endfunction

" -------------------------------------------------------------------- # Utils #

function! s:get_max_widths(msgs, columns)
  let max_widths = map(copy(a:columns), 'strlen(s:config.labels[v:val])')

  for msg in a:msgs
    let widths = map(copy(a:columns), 'strlen(msg[v:val])')
    call map(max_widths, 'max([widths[v:key], v:val])')
  endfor

  return max_widths
endfunction

function! s:get_focused_msg_id()
  try
    return s:trim(split(getline("."), "|")[0])
  catch
    throw "msg not found"
  endtry
endfunction

function! s:refresh_buff_name()
  let buff_name = 'Himalaya'

  if !g:himalaya_hide_done
    let buff_name .= '*'
  endif

  if len(g:himalaya_context) > 0
    let tags = map(copy(g:himalaya_context), 'printf(" +%s", v:val)')
    let buff_name .= join(tags, '')
  endif

  if buff_name != s:buff_name
    execute 'silent! enew'
    execute 'silent! bwipeout ' . s:buff_name
    let s:buff_name = buff_name
  endif
endfunction

function! s:exists_in(list, item)
  return index(a:list, a:item) > -1
endfunction
