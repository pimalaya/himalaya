function! himalaya#shared#bindings#define(bindings)
  for [mode, key, name] in a:bindings
    let plug = substitute(name, "[#_]", "-", "g")
    let plug = printf("<plug>(himalaya-%s)", plug)
    execute printf("%snoremap <silent>%s :call himalaya#%s()<cr>", mode, plug, name)

    if !hasmapto(plug, mode)
      execute printf("%smap <nowait><buffer>%s %s", mode, key, plug)
    endif
  endfor
endfunction
