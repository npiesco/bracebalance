#!/usr/bin/env bash
# Deeply nested shell script - HAS ISSUES (missing closers)
# Comment trap: { [ ( } ] )

_trap1="string trap: { [ ( } ] )"
_trap2=$(cat <<EOF
heredoc trap: { [ ( } ] )
EOF
)

broken_arrays() {
    local -a level1=( 1 2 3 )
    local -a level2=( "${level1[@]"  # <-- missing ] for [@]
    local -a level3=(
        "${level2[@]}"
        4 5 6
    )

    local -A config=(
        [host]="localhost"
        [port]="5432"
    )

    for key in "${!config[@]}"; do
        echo "${config[$key}"  # <-- missing ] for [$key
    done

    printf '%s\n' "${level3[@]}"
}

broken_subshell() {
    local result=$(
        echo "start {"   # <-- { never closed in subshell
        for i in 1 2 3; do
            echo "  item_${i}"
        done
        # missing closing }
    )
    echo "$result"
}

main() {
    broken_arrays
    broken_subshell
# missing }  <-- main never closed
