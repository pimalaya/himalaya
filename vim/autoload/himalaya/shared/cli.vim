function! himalaya#shared#cli#call(cmd, args, log, should_throw)
  call himalaya#shared#log#info(printf("%s…", a:log))
  let cmd = call("printf", ["himalaya --output json " . a:cmd] + a:args)
  let res = system(cmd)

  if empty(res)
    redraw | call himalaya#shared#log#info(printf("%s [OK]", a:log))
  else
    try
      let res = substitute(res, ":null", ":v:null", "g")
      let res = substitute(res, ":true", ":v:true", "g")
      let res = substitute(res, ":false", ":v:false", "g")
      let res = eval(res)
      redraw | call himalaya#shared#log#info(printf("%s [OK]", a:log))
      return res.response
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
