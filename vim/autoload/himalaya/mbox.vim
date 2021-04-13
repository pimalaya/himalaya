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

function! s:input_picker(mboxes)
  let choice = map(copy(a:mboxes), "printf('%s (%d)', v:val, v:key)")
  let choice = input(join(choice, ", ") . ": ")
  redraw | echo
  call himalaya#mbox#post_input(a:mboxes[choice])
endfunction

let s:pickers = {"telescope": function("s:telescope_picker"), "fzf": function("s:fzf_picker"), "input": function("s:input_picker")}

function! himalaya#mbox#input()
  try
    let mboxes = map(s:cli("mailboxes", [], "Fetching mailboxes", 0), "v:val.name")

    if exists("g:himalaya_mailbox_picker") " Get use choice for picker, otherwise check runtimepath
      let l:mailbox_picker = g:himalaya_mailbox_picker
    else
      if &rtp =~ "telescope"
        let l:mailbox_picker = "telescope"
      elseif &rtp =~ "fzf"
        let l:mailbox_picker = "fzf"
      else
        let l:mailbox_picker = "input"
      endif
    endif

    call s:pickers[l:mailbox_picker](mboxes)
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
