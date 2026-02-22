// Deeply nested Swift - HAS ISSUES (missing closers)
// Comment trap: { [ ( } ] )
/* Block trap: { [ ( } ] ) */
let trap1 = """
Triple string trap: { [ ( } ] )
"""

struct BrokenConfig {
    struct Inner {
        struct Deep {
            let values: [(Int, [Int])]
        }
        let deep: Deep
        let extra: [String]
    // missing }  <-- Inner never closed
    let items: [Inner]
}

func brokenProcess(_ input: BrokenConfig) -> [String: Any] {
    let mapped = input.items.map { item -> [String: Any] in
        let result: [String: Any] = [
            "deep": item.deep.values.map { (score, vals) -> [String: Any] in
                [
                    "score": score,
                    "vals":  vals.map { v -> [String: Any] in
                        [
                            "v": v,
                            "doubled": (v * 2,  // <-- missing ) for tuple
                        ]
                    },
                ]
            },
            "extra": item.extra.map { s in
                [
                    "raw": s,
                    "parts": s.split(separator: " ").map { part in
                        ["value": part,
                    // missing ] and } for this dict
                    },
                ]
            },
        ]
        return result
    }
    return ["items": mapped]
// missing }  for func brokenProcess
