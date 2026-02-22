"""Deeply nested Python - HAS ISSUES (missing closers)

Docstring trap: dict({ 'key': [1, 2, (3, 4 } ]
More traps: list([ { ( ] ) }
"""
# Line comment trap: { [ ( } ] )
# Another: data = {"key": [1, (2, 3]}
_TRAP = "string trap: { [ ( } ] )"
_TRAP2 = 'single string trap: dict({ key: [val }'
_TRAP3 = r"raw string trap: [ { ( ] } )"

def broken_function():
    data = {
        "level1": {
            "level2": [
                {
                    "level3": {
                        "level4": [
                            (
                                {"key": [1, 2, 3]},
                                {"key2": [4, 5, 6},  # <-- missing ] for key2 list
                            )
                        ]
                    }
                }
            ]
        }
    }

    result = (
        sorted(
            filter(
                lambda x: (
                    isinstance(x, dict) and
                    all(
                        (k in x) for k in ["a", "b", "c"]
                    )
                ),
                [
                    {"a": 1, "b": [2, (3, 4)], "c": {5: [6, 7]}},
                    {"a": 8, "b": [9, (10, 11], "c": {12: [13, 14]}},  # <-- ) instead of ] for inner tuple
                ]
            ),
            key=lambda x: (x["a"], tuple(x["b"]))
        )
    )

    nested_calls = (
        process(
            transform(
                validate(
                    [
                        (1, 2, (3, 4, [5, 6, (7, 8)])),
                        (9, 10, (11, 12, [13, 14, (15, 16)]))
                    ]
                )
            )
        # <-- missing closing ) for process(
    )

    return data
