// C# broken nested brackets
// Verbatim string with decoy braces must be stripped by sanitizer

using System;
using System.Collections.Generic;

// Verbatim string — all braces inside are decoys
string trap = @"this has {unclosed [brackets and (parens inside the verbatim string";

/* Block comment decoy: { [ ( ignored ) ] } */

class BrokenProcessor
{
    private List<Dictionary<string, int[]>> _layers = new();

    public void Process(Dictionary<string, List<int>> input)
    {
        foreach (var (key, vals) in input)
        {
            var summary = new Dictionary<string, int>
            {
                ["count"] = vals.Count,
                ["sum"]   = vals.Sum(),
            };

            _layers.Add(new Dictionary<string, int[]>
            {
                [key] = summary.Values.ToArray(),
            });

            // <-- missing } for foreach body

    // <-- missing } for Process method

    // <-- missing } for class BrokenProcessor
