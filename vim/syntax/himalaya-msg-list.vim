if exists("b:current_syntax")
  finish
endif

syntax match HimalayaSeparator /│/
syntax match HimalayaHead      /.*\%1l/                         contains=HimalayaSeparator
syntax match HimalayaId        /^.\{-}│/                        contains=HimalayaSeparator
syntax match HimalayaFlags     /^.\{-}│.\{-}│/                  contains=HimalayaId,HimalayaSeparator
syntax match HimalayaSubject   /^.\{-}│.\{-}│.\{-}│/            contains=HimalayaId,HimalayaFlags,HimalayaSeparator
syntax match HimalayaSender    /^.\{-}│.\{-}│.\{-}│.\{-}│/      contains=HimalayaId,HimalayaFlags,HimalayaSubject,HimalayaSeparator
syntax match HimalayaDate      /^.\{-}│.\{-}│.\{-}│.\{-}│.\{-}/ contains=HimalayaId,HimalayaFlags,HimalayaSubject,HimalayaSender,HimalayaSeparator

" FIXME: Find a way to set the line bold AND to keep the style of each columns.
" syntax match HimalayaUnseen /^.\{-}│✷.*$/ contains=HimalayaSeparator
" highlight HimalayaUnseen term=bold cterm=bold gui=bold

highlight HimalayaHead term=bold,underline cterm=bold,underline gui=bold,underline

highlight default link HimalayaSeparator VertSplit
highlight default link HimalayaId        Identifier
highlight default link HimalayaFlags     Special
highlight default link HimalayaSubject   String
highlight default link HimalayaSender    Structure
highlight default link HimalayaDate      Constant

let b:current_syntax = "himalaya-msg-list"
