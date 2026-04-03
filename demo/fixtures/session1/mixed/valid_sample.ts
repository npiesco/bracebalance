// Deeply nested TypeScript - BALANCED
// Comment with braces: { [ ( } ] )
/* Block comment with { unmatched [ braces ( */
const trap1 = "String with { unmatched [ brace";
const trap2 = 'Single-quoted { [ ( traps';
const trap3 = `Template literal with { unmatched } [ ] braces`;
const trap4 = "Escaped quote \" with { brace inside";
const trap5 = `Backtick with \` escaped and { brace`;

interface DeepConfig {
    database: {
        connections: Array<{
            primary: {
                host: string;
                options: {
                    pool: {
                        min: [number, number];
                        max: [number, number];
                        settings: Array<{
                            timeout: [number, [number, number]];
                        }>;
                    };
                };
            };
            replicas: Array<{
                host: string;
                weights: [number[], number[], { computed: [number, number][] }];
            }>;
        }>;
    };
    cache: {
        layers: Array<[string, { size: [number, [number, { unit: string }]] }]>;
    };
}

type DeepNested = {
    [K in string]: {
        children: Array<{
            items: Map<string, {
                values: Set<[number, { nested: Array<[string, number]> }]>;
            }>;
        }>;
    };
};

function processDeep(config: DeepConfig): void {
    const result = Object.entries(config).reduce(
        (acc, [key, value]) => ({
            ...acc,
            [key]: {
                processed: Object.entries(value).map(
                    ([innerKey, innerValue]) => ({
                        key: innerKey,
                        data: Array.isArray(innerValue)
                            ? innerValue.map((item: any) => ({
                                transformed: Object.keys(item).reduce(
                                    (itemAcc, itemKey) => ({
                                        ...itemAcc,
                                        [itemKey]: (() => {
                                            const val = item[itemKey];
                                            return Array.isArray(val)
                                                ? val.map((v: any) => ({
                                                    value: v,
                                                    meta: {
                                                        type: typeof v,
                                                        nested: [v, [v * 2, v * 3]],
                                                    },
                                                }))
                                                : { raw: val };
                                        })(),
                                    }),
                                    {} as Record<string, unknown>
                                ),
                            }))
                            : { value: innerValue },
                    })
                ),
            },
        }),
        {} as Record<string, unknown>
    );

    console.log(JSON.stringify(result, null, 2));
}

class DeepProcessor<T extends Record<string, unknown>> {
    private pipeline: Array<{
        name: string;
        transform: (input: { data: T; meta: { depth: number } }) => {
            result: T;
            stats: { processed: number; errors: Array<{ code: number; msg: string }> };
        };
    }>;

    constructor() {
        this.pipeline = [
            {
                name: "stage1",
                transform: (input) => ({
                    result: Object.fromEntries(
                        Object.entries(input.data).map(([k, v]) => [
                            k,
                            {
                                original: v,
                                transformed: (() => {
                                    if (typeof v === "object" && v !== null) {
                                        return Object.entries(v as Record<string, unknown>).reduce(
                                            (acc, [ik, iv]) => ({
                                                ...acc,
                                                [ik]: [iv, { depth: input.meta.depth + 1 }],
                                            }),
                                            {}
                                        );
                                    }
                                    return v;
                                })(),
                            },
                        ])
                    ) as T,
                    stats: { processed: 1, errors: [] },
                }),
            },
        ];
    }
}
