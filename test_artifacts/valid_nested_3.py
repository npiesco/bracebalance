"""
Deeply nested Python — BALANCED (heavy comment & docstring traps).

The sanitizer must ignore ALL braces inside:
  - # line comments:      { [ ( } ] )
  - "double strings":     { [ ( } ] )
  - 'single strings':     { [ ( } ] )
  - \"\"\"triple-double\"\"\": { [ ( } ] )
  - '''triple-single''':  { [ ( } ] )
  - r"raw double":        { [ ( } ] )
  - r'raw single':        { [ ( } ] )

If any of these leak through, false positives appear.
"""

# Line comment with every brace combo: { [ ( ) ] } {[()]} [({})]{([()])}
_trap1 = "double string: { [ ( } ] ) {[()]}"
_trap2 = 'single string: { [ ( } ] ) {[()]}'
_trap3 = """triple-double with
  multi-line { [ (
  unmatched } ] )
  braces everywhere"""
_trap4 = '''triple-single with
  multi-line { [ (
  unmatched braces
'''
_trap5 = r"raw double: { unmatched [ ( no escapes"
_trap6 = r'raw single: { } [ ] ( )'
_trap7 = "escaped quote: \" still inside string { [ ("
_trap8 = 'escaped single: \' still inside string { [ ('


def build_config():
    """
    Docstring with brace traps:
        config = { "key": [1, (2, 3)] }
        result = sorted([(x, {y: z}) for x in range(10)])
        {{{ [[[ ((( ))) ]]] }}}
    """
    # comment: { unclosed
    config = {
        "database": {  # inline comment: { trap [
            "connections": [
                {
                    "primary": {
                        "host": "localhost",
                        "options": {
                            "pool": {
                                "min": [2, 4],
                                "max": [10, 20],
                                "callbacks": [
                                    (
                                        "on_connect",
                                        {
                                            "retries": [1, 2, 3],
                                            "backoff": {
                                                "params": (
                                                    [100, 200],
                                                    {"jitter": [0.1, 0.5]},
                                                ),
                                            },
                                        },
                                    ),
                                    (
                                        "on_error",
                                        {
                                            "handler": "default",
                                            "meta": ([1, 2], {"nested": [3, 4]}),
                                        },
                                    ),
                                ],
                            }
                        }
                    }
                }
            ]
        },
        "cache": {
            "layers": [
                ("L1", {"size": [1024, (2048, {"unit": "KB"})]}),
                ("L2", {"size": [4096, (8192, {"unit": "MB"})]}),
            ]
        },
    }

    '''
    Another triple-single docstring trap:
    def fake(): { return [1, (2, 3)] }
    for x in range(10): { print([x]) }
    '''

    return config


class DeepProcessor:
    """
    Class docstring:
        processor = DeepProcessor({ "key": [1, 2, 3] })
        result = processor.run([(1, 2), (3, {4: [5]})])
    """

    def __init__(self):
        # comment { trap
        self.pipeline = [
            (
                "stage1",
                {
                    "transforms": [
                        lambda x: (  # { inline trap
                            x * 2
                            if isinstance(x, (int, float))
                            else str(x)  # ) [ trap
                        ),
                        lambda x: (
                            {
                                k: [v * 2 for v in vals]
                                for k, vals in x.items()
                            }
                            if isinstance(x, dict)
                            else x
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
                                [isinstance(i, (int, float)) for i in x]
                            )
                            if isinstance(x, list)
                            else True
                        )
                    ]
                },
            ),
        ]

    def run(self, data):
        """
        Method docstring: { [ ( traps
        result = run({"key": [val for val in (1, 2, 3)]})
        """
        # deep computation: { [
        result = (
            sorted(
                filter(
                    lambda x: (
                        isinstance(x, dict)
                        and all((k in x) for k in ["a", "b"])
                    ),
                    [
                        {"a": 1, "b": [2, (3, 4)], "c": {5: [6, 7]}},
                        {"a": 8, "b": [9, (10, 11)], "c": {12: [13]}},
                    ],
                ),
                key=lambda x: (x["a"], tuple(x["b"])),
            )
        )
        return result
