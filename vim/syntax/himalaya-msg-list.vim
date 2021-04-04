if exists("b:current_syntax")
  finish
endif

syntax match hya_sep     /|/
syntax match hya_uid     /^|.\{-}|/                         contains=hya_sep
syntax match hya_flags   /^|.\{-}|.\{-}|/                   contains=hya_uid,hya_sep
syntax match hya_subject /^|.\{-}|.\{-}|.\{-}|/             contains=hya_uid,hya_flags,hya_sep
syntax match hya_sender  /^|.\{-}|.\{-}|.\{-}|.\{-}|/       contains=hya_uid,hya_flags,hya_subject,hya_sep
syntax match hya_date    /^|.\{-}|.\{-}|.\{-}|.\{-}|.\{-}|/ contains=hya_uid,hya_flags,hya_subject,hya_sender,hya_sep
syntax match hya_head    /.*\%1l/                           contains=hya_sep
syntax match hya_unseen  /^|.\{-}|N.*$/                     contains=hya_sep

highlight default link hya_sep     VertSplit
highlight default link hya_uid     Identifier
highlight default link hya_flags   Special
highlight default link hya_subject String
highlight default link hya_sender  Structure
highlight default link hya_date    Constant

highlight hya_head   term=bold,underline cterm=bold,underline gui=bold,underline
highlight hya_unseen term=bold           cterm=bold           gui=bold

let b:current_syntax = "himalaya-msg-list"
