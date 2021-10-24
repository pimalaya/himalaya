function! himalaya#request#json(opts)
  let msg = get(a:, 'opts.msg', '')
  let cmd = get(a:, 'opts.cmd', '')
  let args = get(a:, 'opts.args', [])
  let should_throw = get(a:, 'opts.should_throw', v:false)

  call himalaya#shared#log#info(printf('%s…', msg))
  let cmd = call('printf', ['himalaya --output json ' . cmd] + args)
  let res = system(cmd)

  if empty(res)
    redraw | call himalaya#shared#log#info(printf('%s [OK]', msg))
  else
    try
      let res = substitute(res, ':null', ':v:null', 'g')
      let res = substitute(res, ':true', ':v:true', 'g')
      let res = substitute(res, ':false', ':v:false', 'g')
      let res = eval(res)
      redraw | call himalaya#shared#log#info(printf('%s [OK]', msg))
      return res.response
    catch
      redraw
      for line in split(res, '\n')
        call himalaya#shared#log#err(line)
      endfor
      if should_throw
        throw ''
      endif
    endtry
  endif
endfunction

function! himalaya#request#plain(opts)
  let msg = get(a:opts, 'msg', '')
  let cmd = get(a:opts, 'cmd', '')
  let args = get(a:opts, 'args', [])
  let should_throw = get(a:, 'opts.should_throw', v:false)

  call himalaya#shared#log#info(printf('%s…', msg))
  let cmd = call('printf', ['himalaya --output plain ' . cmd] + args)
  let res = system(cmd)

  if empty(res)
    redraw | call himalaya#shared#log#info(printf('%s [OK]', msg))
  else
    try
      redraw | call himalaya#shared#log#info(printf('%s [OK]', msg))
      return trim(res)
    catch
      redraw
      for line in split(res, '\n')
        call himalaya#shared#log#err(line)
      endfor
      if should_throw
        throw ''
      endif
    endtry
  endif
endfunction
