# Deeply nested Ruby - BALANCED
# Comment trap: { [ ( } ] )

_trap1 = "string trap: { [ ( } ] )"
_trap2 = 'single trap: { [ ( } ] )'
_trap3 = %q{ trap: ( [ { unbalanced openers inside }
_trap4 = %q[ trap: { ( [ unbalanced openers inside ]
_trap5 = %w[ word1 { word2 [ word3 ( word4 ) ]
_trap6 = <<~HEREDOC
  heredoc trap: { [ ( } ] )
  more: vec![ { ( ]
HEREDOC
_trap7 = <<~'HEREDOC'
  unexpanded heredoc: { [ ( } ] )
HEREDOC

def deep_config
  {
    database: {
      connections: [
        {
          primary: {
            host: "localhost",
            options: {
              pool: {
                min: [2, 4],
                max: [10, 20],
                settings: [
                  { timeout: [30, [60, 90]] },
                  { retry:   [1,  [2,  3]] },
                ],
              },
            },
          },
          replicas: [
            {
              host: "replica1",
              weights: [
                [1, 2, 3],
                [4, 5, 6],
                {
                  computed: [
                    [1.max(2), 3.min(4)],
                    [5.max(6), 7.min(8)],
                  ],
                },
              ],
            },
          ],
        },
      ],
    },
    cache: {
      layers: [
        ["L1", { size: [1024, [2048, { unit: "KB" }]] }],
        ["L2", { size: [4096, [8192, { unit: "MB" }]] }],
      ],
    },
  }
end

def process(input)
  input.each_with_object({}) do |(k, v), acc|
    acc[k] = {
      transformed: v.is_a?(Array) ? v.map { |item|
        {
          value: item,
          meta: {
            keys: item.is_a?(Hash) ? item.keys.map { |key|
              { key => <<~LABEL
                  meta-#{key}: { [ ( is a trap }
                LABEL
              }
            } : [],
          },
        }
      } : { raw: v },
    }
  end
end
