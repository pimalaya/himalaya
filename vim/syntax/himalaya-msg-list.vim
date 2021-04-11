if exists("b:current_syntax")
  finish
endif

syntax match hym_sep     /|/
syntax match hym_uid     /^|.\{-}|/                         contains=hym_sep
syntax match hym_flags   /^|.\{-}|.\{-}|/                   contains=hym_uid,hym_sep
syntax match hym_subject /^|.\{-}|.\{-}|.\{-}|/             contains=hym_uid,hym_flags,hym_sep
syntax match hym_sender  /^|.\{-}|.\{-}|.\{-}|.\{-}|/       contains=hym_uid,hym_flags,hym_subject,hym_sep
syntax match hym_date    /^|.\{-}|.\{-}|.\{-}|.\{-}|.\{-}|/ contains=hym_uid,hym_flags,hym_subject,hym_sender,hym_sep
syntax match hym_head    /.*\%1l/                           contains=hym_sep
syntax match hym_unseen  /^|.\{-}|N.*$/                     contains=hym_sep

highlight hym_head   term=bold,underline cterm=bold,underline gui=bold,underline
highlight hym_unseen term=bold           cterm=bold           gui=bold

highlight default link hym_sep     VertSplit
highlight default link hym_uid     Identifier
highlight default link hym_flags   Special
highlight default link hym_subject String
highlight default link hym_sender  Structure
highlight default link hym_date    Constant

let b:current_syntax = "himalaya-msg-list"
