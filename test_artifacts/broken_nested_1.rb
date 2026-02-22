# Deeply nested Ruby - HAS ISSUES (missing closers)
# Comment trap: { [ ( } ] )

_trap1 = "string trap: { [ ( } ] )"
_trap2 = %q{ percent-q trap: { [ ( } ] ) }
_trap3 = <<~HEREDOC
  heredoc trap: { [ ( } ] )
HEREDOC

def broken
  data = {
    level1: {
      level2: {
        level3: {
          level4: [
            {key: [1, 2, {nested: [3, 4},  # <-- missing ] for nested
            {key: [5, 6]},
          ]
        }
      }
    }
  }

  result = [
    {
      name: "a",
      vals: [1, [2, (3, 4)],  # <-- ) instead of ] for inner
    },
    {
      name: "b",
      vals: [5, [6, [7, 8]]],
    },
  ]

  nested = {
    a: {
      b: {
        c: [1, [2, [3, [4, 5]    # <-- missing 3 closing ]
      }
    }
  }

  [data, result, nested]
end

_unclosed = {  # intentionally unclosed { — unambiguous break
