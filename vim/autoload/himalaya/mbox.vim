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

function! himalaya#mbox#input()
  try
    let mboxes = map(s:cli("mailboxes", [], "Fetching mailboxes"), "v:val.name")
    if &rtp =~ "fzf"
      call fzf#run({
        \"source": mboxes,
        \"sink": function("himalaya#mbox#post_input"),
        \"down": "25%",
      \})
    else
      let choice = map(copy(mboxes), "printf('%s (%d)', v:val, v:key)")
      let choice = input(join(choice, ", ") . ": ")
      redraw | echo
      call himalaya#mbox#post_input(mboxes[choice])
    endif
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
