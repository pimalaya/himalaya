function! himalaya#shared#cli#call(cmd, args, log)
  call himalaya#shared#log#info(printf("%s…", a:log))
  let cmd = call("printf", ["himalaya --output json " . a:cmd] + a:args)
  let res = system(cmd)

  if empty(res)
    redraw | call himalaya#shared#log#info(printf("%s [OK]", a:log))
  else
    try
      let res = eval(res)
      redraw | call himalaya#shared#log#info(printf("%s [OK]", a:log))
      return res
    catch
      redraw | call himalaya#shared#log#info(printf("%s [ERR]", a:log))
      for line in split(res, "\n")
        call himalaya#shared#log#err(line)
      endfor
      throw ""
    endtry
  endif
endfunction
