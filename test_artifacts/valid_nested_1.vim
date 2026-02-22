" Deeply nested Vim script - BALANCED
" Comment trap: { [ ( } ] )
" More traps: let x = { 'key': [1, (2, 3] }

function! DeepConfig() abort
    " Comment inside function: { [ ( } ] )
    let config = {
        \ 'database': {
        \   'connections': [
        \     {
        \       'primary': {
        \         'host': 'localhost',
        \         'options': {
        \           'pool': {
        \             'min': [2, 4],
        \             'max': [10, 20],
        \             'settings': [
        \               {'timeout': [30, [60, 90]]},
        \               {'retry':   [1,  [2,  3]]},
        \             ],
        \           },
        \         },
        \       },
        \       'replicas': [
        \         {
        \           'host': 'replica1',
        \           'weights': [[1,2,3], [4,5,6]],
        \         },
        \       ],
        \     },
        \   ],
        \ },
        \ 'cache': {
        \   'layers': [
        \     ['L1', {'size': [1024, [2048]]}],
        \     ['L2', {'size': [4096, [8192]]}],
        \   ],
        \ },
    \ }
    return config
endfunction

function! ProcessConfig(config) abort
    " Process each top-level key: { [ ( } ] )
    let result = {}
    for [key, val] in items(a:config)
        " Inner comment: { [ ( } ] )
        if type(val) == v:t_list
            let result[key] = map(copy(val), {_, v ->
                \ type(v) == v:t_dict
                \   ? {'processed': keys(v), 'count': len(v)}
                \   : {'raw': v}
                \ })
        elseif type(val) == v:t_dict
            let result[key] = {
                \ 'keys':   keys(val),
                \ 'values': values(val),
                \ 'nested': map(copy(val), {k, v -> [k, v]}),
            \ }
        else
            let result[key] = val
        endif
    endfor
    return result
endfunction
