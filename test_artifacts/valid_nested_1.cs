// C# valid nested brackets
// Tests: verbatim strings (@"..."), line/block comments, nested collections

using System;
using System.Collections.Generic;
using System.Linq;

// Verbatim string with decoy braces — sanitizer must strip all content
string verbatim = @"Say ""hello"" to {world}, check [items] and (stuff)";

/* Block comment { [ ( these must be ignored ) ] } */

class DataPipeline<T>
{
    private readonly Dictionary<string, List<T>> _store = new();

    public void Add(string key, List<T> items)
    {
        if (!_store.ContainsKey(key))
        {
            _store[key] = new List<T>();
        }
        _store[key].AddRange(items);
    }

    public Dictionary<string, int> Summarise(Func<T, int> selector)
    {
        return _store.ToDictionary(
            kvp => kvp.Key,
            kvp => kvp.Value.Select(selector).Sum()
        );
    }
}

class Program
{
    static void Main()
    {
        var pipe = new DataPipeline<int>();
        pipe.Add("a", new List<int> { 1, 2, 3 });
        pipe.Add("b", new List<int> { 4, 5, 6 });

        var nested = new Dictionary<string, Dictionary<string, List<int>>>
        {
            ["x"] = new Dictionary<string, List<int>>
            {
                ["p"] = new List<int> { 10, 20 },
                ["q"] = new List<int> { 30, 40 },
            },
            ["y"] = new Dictionary<string, List<int>>
            {
                ["r"] = new List<int> { 50 },
            },
        };

        foreach (var (k, inner) in nested)
        {
            foreach (var (ik, iv) in inner)
            {
                Console.WriteLine($"{k}.{ik}: [{string.Join(", ", iv)}]");
            }
        }
    }
}
