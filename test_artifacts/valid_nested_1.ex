defmodule ValidNested do
  @moduledoc """
  Deeply nested Elixir module - BALANCED
  Docstring trap: { [ ( } ] )
  Sigil trap: ~s"{ [ ( } ] )"
  """

  # Line comment trap: { [ ( } ] )

  @trap1 ~s"{ [ ( sigil string trap } ] )"
  @trap2 ~r/{ regex trap [ ( } ] )/
  @trap3 ~w{ word1 { word2 }  # ~w sigil - braces are delimiters not structure
  @trap4 "double string trap: { [ ( } ] )"

  @spec process(map()) :: map()
  def process(config) do
    config
    |> Map.get(:database, %{})
    |> then(fn db ->
      %{
        connections: Enum.map(Map.get(db, :connections, []), fn conn ->
          %{
            primary: %{
              host: conn.host,
              options: %{
                pool: %{
                  min: {2, 4},
                  max: {10, 20},
                  settings: [
                    %{timeout: {30, {60, 90}}},
                    %{retry:   {1,  {2,  3}}},
                  ],
                },
              },
            },
            replicas: Enum.map(conn.replicas, fn r ->
              %{
                host: r.host,
                weights: Enum.map(r.weights, fn w ->
                  %{value: w, meta: %{type: ~s"weight", scaled: {w * 2, w * 3}}}
                end),
              }
            end),
          }
        end),
        cache: %{
          layers: Enum.map(Map.get(db, :cache_layers, []), fn {name, size} ->
            %{
              name: name,
              size: size,
              stats: %{
                hits:   {0, ~r/\d+/},
                misses: {0, ~r/\d+/},
              },
            }
          end),
        },
      }
    end)
  end

  defp validate(%{entries: entries} = _data) do
    Enum.reduce(entries, {:ok, []}, fn entry, {_status, acc} ->
      case entry do
        %{type: :valid, values: vals} ->
          {:ok, [{:valid, vals} | acc]}
        %{type: :nested, children: kids} ->
          result = Enum.map(kids, fn kid ->
            %{
              id: kid.id,
              computed: {kid.score, kid.score * 2, kid.score * 3},
              label: ~s"kid-#{kid.id}",
            }
          end)
          {:ok, [{:nested, result} | acc]}
        _ ->
          {:error, acc}
      end
    end)
  end
end
