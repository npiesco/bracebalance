"""Deeply nested Python - BALANCED"""

CONFIG = {
    "database": {
        "connections": [
            {
                "primary": {
                    "host": "localhost",
                    "options": {
                        "pool": {
                            "min": (2, 4),
                            "max": (10, 20),
                            "settings": [
                                {"timeout": [30, (60, 90)]},
                                {"retry": [1, (2, 3)]},
                            ]
                        }
                    }
                },
                "replicas": [
                    {
                        "host": "replica1",
                        "weights": (
                            [1, 2, 3],
                            [4, 5, 6],
                            {
                                "computed": [
                                    (sum([1, 2]), max([3, 4])),
                                    (min([5, 6]), len([7, 8, 9])),
                                ]
                            }
                        )
                    }
                ]
            }
        ]
    },
    "cache": {
        "layers": [
            (
                "L1",
                {"size": [1024, (2048, {"unit": "KB"})]},
            ),
            (
                "L2",
                {"size": [4096, (8192, {"unit": "MB"})]},
            ),
        ]
    }
}


def deep_comprehension():
    return {
        k: [
            (
                v,
                {
                    "processed": [
                        (
                            item,
                            [
                                sub for sub in (
                                    range(len(str(item)))
                                )
                            ]
                        )
                        for item in v
                    ]
                }
            )
            for v in values
        ]
        for k, values in {
            "a": [[1, 2], [3, 4]],
            "b": [[5, 6], [7, 8]],
        }.items()
    }
