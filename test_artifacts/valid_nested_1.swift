// Deeply nested Swift - BALANCED
// Comment trap: { [ ( } ] )
/* Block comment trap: { [ ( } ] ) */

let trap1 = "string trap: { [ ( } ] )"
let trap2 = """
Triple string trap: { [ ( } ] )
More: func foo() { return [1, (2, 3)] }
"""
let trap3 = 'x' // single char literal, not a string

struct DeepConfig {
    struct Database {
        struct Connection {
            struct Options {
                struct Pool {
                    let min: (Int, Int)
                    let max: (Int, Int)
                    let settings: [Setting]
                }
                struct Setting {
                    let timeout: (Int, (Int, Int))
                    let retry:   (Int, (Int, Int))
                }
                let pool: Pool
            }
            let host: String
            let options: Options
            let replicas: [Replica]
        }
        struct Replica {
            let host: String
            let weights: [[Int]]
        }
        let connections: [Connection]
    }
    struct Cache {
        struct Layer {
            let name: String
            let size: (Int, (Int, (String, String)))
        }
        let layers: [Layer]
    }
    let database: Database
    let cache: Cache
}

func process(_ config: DeepConfig) -> [String: Any] {
    let connections = config.database.connections.map { conn -> [String: Any] in
        let options: [String: Any] = [
            "pool": [
                "min": conn.options.pool.min,
                "max": conn.options.pool.max,
                "settings": conn.options.pool.settings.map { s -> [String: Any] in
                    [
                        "timeout": s.timeout,
                        "retry":   s.retry,
                    ]
                },
            ],
        ]
        return [
            "host":     conn.host,
            "options":  options,
            "replicas": conn.replicas.map { r in
                [
                    "host":    r.host,
                    "weights": r.weights.map { $0.reduce(0, +) },
                ]
            },
        ]
    }
    let layers = config.cache.layers.map { layer -> [String: Any] in
        [
            "name": layer.name,
            "size": layer.size,
            "label": """
                layer-\(layer.name): { [ trap }
                """,
        ]
    }
    return ["connections": connections, "cache": ["layers": layers]]
}
