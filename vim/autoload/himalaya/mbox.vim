let s:dir = expand("<sfile>:h")
let s:cli = function("himalaya#shared#cli#call")

" Pagination

let s:curr_page = 0
function! himalaya#mbox#curr_page()
  return s:curr_page
endfunction

function! himalaya#mbox#prev_page()
  let s:curr_page = max([0, s:curr_page - 1])
  call himalaya#msg#list()
endfunction

function! himalaya#mbox#next_page()
  let s:curr_page = s:curr_page + 1
  call himalaya#msg#list()
endfunction

" Mailbox

let s:curr_mbox = "INBOX"
function! himalaya#mbox#curr_mbox()
  return s:curr_mbox
endfunction

function! s:telescope_picker(mboxes)
  call luaeval('require("himalaya.mbox").mbox_picker')(a:mboxes)
endfunction

function! s:fzf_picker(mboxes)
  call fzf#run({
    \"source": a:mboxes,
    \"sink": function("himalaya#mbox#post_input"),
    \"down": "25%",
  \})
endfunction

function! s:native_picker(mboxes)
  let choice = map(copy(a:mboxes), "printf('%s (%d)', v:val, v:key)")
  let choice = input(join(choice, ", ") . ": ")
  redraw | echo
  call himalaya#mbox#post_input(a:mboxes[choice])
endfunction

let s:pickers = {
  \"telescope": function("s:telescope_picker"),
  \"fzf": function("s:fzf_picker"),
  \"native": function("s:native_picker"),
\}

function! himalaya#mbox#input()
  try
    let mboxes = map(s:cli("mailboxes", [], "Fetching mailboxes", 0), "v:val.name")

    " Get user choice for picker, otherwise check runtimepath
    if exists("g:himalaya_mailbox_picker")
      let mbox_picker = g:himalaya_mailbox_picker
    else
      if &rtp =~ "telescope"
        let mbox_picker = "telescope"
      elseif &rtp =~ "fzf"
        let mbox_picker = "fzf"
      else
        let mbox_picker = "native"
      endif
    endif

    call s:pickers[mbox_picker](mboxes)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#mbox#post_input(mbox)
  let s:curr_mbox = a:mbox
  let s:curr_page = 0
  call himalaya#msg#list()
endfunction
