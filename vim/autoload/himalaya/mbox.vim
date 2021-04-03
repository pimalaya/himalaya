let s:print_info = function("himalaya#utils#print_msg")
let s:print_err = function("himalaya#utils#print_err")
let s:cli = function("himalaya#shared#cli")

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
    call s:print_info("Fetching mailboxes…")

    let mboxes = map(s:cli("mailboxes", []), "v:val.name")

    " if &rtp =~ "fzf.vim"
    "   call fzf#run({
    "     \"source": mboxes,
    "     \"sink": function("himalaya#mbox#post_input"),
    "     \"down": "25%",
    "   \})
    " else
      let choice = map(copy(mboxes), "printf('%s (%d)', v:val, v:key)")
      redraw | echo
      let choice = input(join(choice, ", ") . ": ")
      redraw | echo
      call himalaya#mbox#post_input(mboxes[choice])
    " endif
  catch
    call s:print_err(v:exception)
  endtry
endfunction

function! himalaya#mbox#post_input(mbox)
  let s:curr_mbox = a:mbox
  let s:curr_page = 0
  call himalaya#msg#list()
endfunction
