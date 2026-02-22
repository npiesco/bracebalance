" Deeply nested Vim script - HAS ISSUES (missing closers)
" Comment trap: { [ ( } ] )
" More traps: let x = { 'key': [1, (2, 3] }

function! BrokenConfig() abort
    " Comment inside: { [ ( } ] )
    let config = {
        \ 'level1': {
        \   'level2': {
        \     'level3': [
        \       {
        \         'values': [1, [2, [3, 4],  " <-- missing ] for level-3 array
        \       },
        \     ],
        \   },
        \ },
    \ }

    let nested = [
        \ [1, 2, 3],
        \ [4, [5, [6, 7]],
        " <-- missing ] for outer nested array and ] for the level-2 array

    return config
endfunction

function! BrokenProcess(data) abort
    let result = {}
    for [k, v] in items(a:data)
        let result[k] = {
            \ 'transformed': map(copy(v), {_, x -> {'value': x, 'meta': [x * 2,}),
            " <-- missing ] for meta array and } for map lambda and ) for map(
        \ }
    endfor
    return result
" missing endfunction
