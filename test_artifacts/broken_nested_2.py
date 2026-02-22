"""
Deeply nested Python — HAS ISSUES (comments & docstrings hide real errors).

This file has genuine balance errors buried among comment/string traps.
The sanitizer must strip the traps but leave the real mismatches.

Docstring trap: config = { "key": [1, (2, 3)] }
More traps: {{{ [[[ ((( ))) ]]] }}}
"""

# Line comment trap: { [ ( } ] )
_trap1 = "string trap: { [ ( } ] )"
_trap2 = 'single trap: { [ ( } ] )'
_trap3 = """triple trap: {
  multi-line [ (
  unmatched } ] )
"""
_trap4 = r"raw trap: { unmatched [ ("


def broken_with_comments():
    """
    Function docstring:
        data = { "key": [1, (2, 3)] }
        result = sorted([(x, {y: z}) for x in range(10)])
    """
    # comment: { [ ( — fine
    config = {
        "database": {
            "connections": [
                {
                    "primary": {
                        "host": "localhost",  # comment { trap
                        "options": {
                            "pool": {
                                "min": [2, 4],
                                "max": [10, 20,  # <-- missing ] for max list
                                "settings": [
                                    {
                                        "timeout": [30, [60, 90]],
                                    },
                                ],
                            }
                        }
                    }
                }
            ]
        },
    }

    '''
    Triple-single trap:
    def fake(): { return [1, (2, 3)] }
    '''

    # comment: fine { [
    result = (
        sorted(
            filter(
                lambda x: (
                    isinstance(x, dict) and
                    all(
                        (k in x) for k in ["a", "b"]
                    )
                ),
                [
                    {"a": 1, "b": [2, (3, 4)], "c": {5: [6, 7]}},
                    {"a": 8, "b": [9, (10, 11], "c": {12: [13, 14]}},  # <-- ) missing, ] instead
                ]
            ),
            key=lambda x: (x["a"], tuple(x["b"])
        )  # <-- missing ) for tuple() call
    )

    return config


class BrokenProcessor:
    """
    Class docstring:
        processor = BrokenProcessor({ "key": [val for val in (1, 2, 3)] })
        { [ ( ) ] }
    """

    def __init__(self):
        # comment { trap
        self.pipeline = [
            (
                "stage1",
                {
                    "transforms": [
                        lambda x: (
                            x * 2
                            if isinstance(x, (int, float))
                            else str(x)
                        ),
                    ]
                },
            ),
            (
                "stage2",
                {
                    "validators": [
                        lambda x: (
                            all(
                                [isinstance(i, (int, float) for i in x]  # <-- missing ) for isinstance
                            )
                            if isinstance(x, list)
                            else True
                        )
                    ]
                },
            ),
        ]
