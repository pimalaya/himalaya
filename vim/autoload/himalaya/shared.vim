function! himalaya#shared#define_bindings(bindings)
  for [mode, key, name] in a:bindings
    let plug = substitute(name, "[#_]", "-", "g")
    let plug = printf("<plug>(himalaya-%s)", plug)
    execute printf("%snoremap <silent>%s :call himalaya#%s()<cr>", mode, plug, name)

    if !hasmapto(plug, mode)
      execute printf("%smap <nowait><buffer>%s %s", mode, key, plug)
    endif
  endfor
endfunction

function! himalaya#shared#cli(cmd, args)
  let cmd = call("printf", ["himalaya --output json " . a:cmd] + a:args)
  let res = system(cmd)

  if !empty(res)
    try
      return eval(res)
    catch
      throw res
    endtry
  endif
endfunction

function! himalaya#shared#thread_fold(lnum)
  return getline(a:lnum)[0] == ">"
endfunction
