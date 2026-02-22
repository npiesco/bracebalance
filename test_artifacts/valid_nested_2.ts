// Deeply nested TypeScript - BALANCED
// Comment traps: { [ ( } ] )
/* Block { [ ( */
const trap = "{ string [ brace ( trap";
const trap2 = `template { literal [ trap`;

type RecursiveTree<T> = {
    value: T;
    children: Array<{
        node: RecursiveTree<T>;
        metadata: {
            path: [string, ...string[]];
            attributes: Map<string, {
                scores: [number, number, number];
                nested: { deep: { deeper: Array<[string, T]> } };
            }>;
        };
    }>;
};

async function fetchDeepData(): Promise<{
    results: Array<{
        id: number;
        layers: Array<{
            data: Map<string, {
                items: Set<[number, { tags: string[] }]>;
            }>;
        }>;
    }>;
}> {
    const response = await fetch("/api/deep");
    const raw = await response.json();

    return {
        results: raw.map((entry: any) => ({
            id: entry.id,
            layers: entry.layers.map((layer: any) => ({
                data: new Map(
                    Object.entries(layer).map(([key, val]: [string, any]) => [
                        key,
                        {
                            items: new Set(
                                val.items.map((item: any) => [
                                    item.value,
                                    {
                                        tags: item.tags.filter((t: string) =>
                                            t.length > 0 && (
                                                t.startsWith("important") ||
                                                t.includes("critical")
                                            )
                                        ),
                                    },
                                ] as [number, { tags: string[] }])
                            ),
                        },
                    ])
                ),
            })),
        })),
    };
}

const deepReducer = (
    state: {
        entities: Record<string, {
            data: Array<[number, { nested: { values: number[] } }]>;
        }>;
    },
    action: {
        type: string;
        payload: {
            updates: Array<{
                id: string;
                changes: Array<{
                    field: string;
                    value: [number, { meta: { source: string } }];
                }>;
            }>;
        };
    }
): typeof state => {
    switch (action.type) {
        case "UPDATE":
            return {
                entities: {
                    ...state.entities,
                    ...Object.fromEntries(
                        action.payload.updates.map((update) => [
                            update.id,
                            {
                                data: update.changes.map((change) => [
                                    change.value[0],
                                    {
                                        nested: {
                                            values: [
                                                change.value[0],
                                                change.value[0] * 2,
                                                change.value[0] * 3,
                                            ],
                                        },
                                    },
                                ] as [number, { nested: { values: number[] } }]),
                            },
                        ])
                    ),
                },
            };
        default:
            return state;
    }
};
