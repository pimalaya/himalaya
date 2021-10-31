" Account

let s:curr_account = ""

function! himalaya#account#curr()
  return s:curr_account
endfunction

function! himalaya#account#set(account)
  let s:curr_account = a:account
endfunction
