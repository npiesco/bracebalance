#!/usr/bin/env bash
# Deeply nested shell script - BALANCED
# Comment trap: { [ ( } ] )

_trap1="string trap: { [ ( } ] )"
_trap2='single trap: { [ ( } ] )'
_trap3=$(cat <<EOF
heredoc trap: { [ ( } ] )
more: array=( { [ trap )
EOF
)
_trap4=$(cat <<'NOEXPAND'
unexpanded heredoc: { [ ( } ] )
NOEXPAND
)

deep_process() {
    local config="$1"
    local result=()

    while IFS= read -r line; do
        local parts
        IFS=',' read -ra parts <<< "$line"
        local entry=()
        for part in "${parts[@]}"; do
            local kv
            IFS='=' read -ra kv <<< "$part"
            entry+=( ["${kv[0]}"]="${kv[1]}" )
        done
        result+=( "$(declare -p entry)" )
    done <<BLOCK
key1=val1,key2=val2
key3=val3,key4=val4
BLOCK

    printf '%s\n' "${result[@]}"
}

nested_arrays() {
    local -a level1=( 1 2 3 )
    local -a level2=( "${level1[@]}" 4 5 6 )
    local -a level3=(
        "${level2[@]}"
        $(echo "7 8 9")
    )
    local -A config=(
        [host]="localhost"
        [port]="5432"
        [options]="pool_min=2,pool_max=10"
    )
    for key in "${!config[@]}"; do
        local val="${config[$key]}"
        echo "  ${key}=${val}"
    done
    printf '%s\n' "${level3[@]}"
}

main() {
    local output
    output=$(deep_process "$(cat <<INPUT
a=1,b=2
c=3,d=4
INPUT
    )")
    nested_arrays
    echo "$output"
}

main "$@"
