"""Deeply nested Python - BALANCED"""

def outer():
    data = {
        "users": [
            {
                "name": "Alice",
                "scores": [
                    (10, 20, 30),
                    (40, 50, 60),
                ],
                "metadata": {
                    "tags": [
                        ("admin", {"level": [1, 2, 3]}),
                        ("editor", {"level": [4, 5, 6]}),
                    ],
                    "nested": {
                        "deep": {
                            "deeper": {
                                "deepest": [
                                    (
                                        {"key": [1, (2, 3), [4, 5]]},
                                        {"key2": [6, (7, 8), [9, 10]]},
                                    )
                                ]
                            }
                        }
                    }
                }
            }
        ]
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
                    {"a": 8, "b": [9, (10, 11)], "c": {12: [13, 14]}},
                ]
            ),
            key=lambda x: (x["a"], tuple(x["b"]))
        )
    )

    return (data, result)


class Processor:
    def __init__(self):
        self.pipeline = [
            (
                "stage1",
                {
                    "transforms": [
                        lambda x: (
                            x * 2 if isinstance(x, (int, float)) else str(x)
                        ),
                        lambda x: (
                            {k: [v * 2 for v in vals] for k, vals in x.items()}
                            if isinstance(x, dict)
                            else x
                        ),
                    ]
                }
            ),
            (
                "stage2",
                {
                    "validators": [
                        lambda x: (
                            all(
                                [isinstance(i, (int, float)) for i in x]
                            ) if isinstance(x, list) else True
                        )
                    ]
                }
            )
        ]
