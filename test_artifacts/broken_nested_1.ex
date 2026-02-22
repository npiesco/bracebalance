defmodule BrokenNested do
  @moduledoc """
  Deeply nested Elixir module - HAS ISSUES (missing closers)
  Docstring trap: { [ ( } ] )
  """

  # Comment trap: { [ ( } ] )
  @trap1 ~s"{ [ ( sigil string trap } ] )"
  @trap2 ~r/{ regex trap }/

  def broken(config) do
    result = config
    |> Map.get(:layers, [])
    |> Enum.map(fn layer ->
      %{
        name: layer.name,
        items: Enum.map(layer.items, fn item ->
          %{
            id: item.id,
            values: {item.score, [item.score * 2,  # <-- missing ] and }
          }
        end)
      }
    end)

    nested = %{
      a: %{
        b: %{
          c: [1, [2, [3, [4, 5]    # <-- missing 3 closing ]
        }
      }
    }

    {result, nested}
  # missing end for def broken
end
