let s:dir = expand("<sfile>:h")
let s:cli = function("himalaya#shared#cli#call")

" Pickers

function! s:telescope_picker(cb, mboxes)
  call luaeval("require('himalaya.mbox').mbox_picker")(a:cb, a:mboxes)
endfunction

function! s:fzf_picker(cb, mboxes)
  call fzf#run({
    \"source": a:mboxes,
    \"sink": function(a:cb),
    \"down": "25%",
  \})
endfunction

function! s:native_picker(cb, mboxes)
  let choice = map(copy(a:mboxes), "printf('%s (%d)', v:val, v:key)")
  let choice = input(join(choice, ", ") . ": ")
  redraw | echo
  call function(a:cb)(a:mboxes[choice])
endfunction

" Pagination

let s:curr_page = 1

function! himalaya#mbox#curr_page()
  return s:curr_page
endfunction

function! himalaya#mbox#prev_page()
  let s:curr_page = max([1, s:curr_page - 1])
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

function! himalaya#mbox#pick(cb)
  try
    let account = himalaya#account#curr()
    let mboxes = map(s:cli("--account %s mailboxes", [shellescape(account)], "Fetching mailboxes", 0), "v:val.name")

    if exists("g:himalaya_mailbox_picker")
      let picker = g:himalaya_mailbox_picker
    else
      if &rtp =~ "telescope"
        let picker = "telescope"
      elseif &rtp =~ "fzf"
        let picker = "fzf"
      else
        let picker = "native"
      endif
    endif

    execute printf("call s:%s_picker(a:cb, mboxes)", picker)
  catch
    if !empty(v:exception)
      redraw | call himalaya#shared#log#err(v:exception)
    endif
  endtry
endfunction

function! himalaya#mbox#change()
  call himalaya#mbox#pick("himalaya#mbox#_change")
endfunction

function! himalaya#mbox#_change(mbox)
  let s:curr_mbox = a:mbox
  let s:curr_page = 1
  call himalaya#msg#list()
endfunction
