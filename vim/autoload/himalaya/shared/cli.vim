function! himalaya#shared#cli#call(cmd, args, log, should_throw, ...)
  call himalaya#shared#log#info(printf("%s…", a:log))
  if a:0 == 0
    let cmd = call("printf", ["himalaya --output json " . a:cmd] + a:args)
  else
    let cmd = call("printf", ["himalaya --output " . a:1 . " " . a:cmd] + a:args)
  endif
  let res = system(cmd)

  if empty(res)
    redraw | call himalaya#shared#log#info(printf("%s [OK]", a:log))
  else
    try
      if a:0 == 0
        let res = eval(res)
        redraw | call himalaya#shared#log#info(printf("%s [OK]", a:log))
        return res.response
      elseif a:1 == "plain"
        redraw | call himalaya#shared#log#info(printf("%s [OK]", a:log))
        return res
      else
        call himalaya#shared#log#err("Unsupported cli output format requested")
      end
    catch
      redraw
      for line in split(res, "\n")
        call himalaya#shared#log#err(line)
      endfor
      if a:should_throw
        throw ""
      endif
    endtry
  endif
endfunction
