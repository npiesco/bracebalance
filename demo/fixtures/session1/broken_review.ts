// Deeply nested TypeScript - HAS ISSUES (missing closers)
// Comment trap: { [ ( } ] )
/* Block comment trap:
 *   const x: Array<{items: [number, {key: string}]> = [;
 *   interface Fake { broken: [number, (string };
 */
/**
 * @param config - `{ data: Array<[string, {nested: [number]}> }`
 * @returns `{ result: Array<(number, [string])> }`
 */
const _trap = "string trap: { [ ( } ] )";
const _trap2 = `template trap: ${ "{ [ (" } ] )`;
const _trap3 = 'single trap: { [ ( } ] )';

interface BrokenConfig {
    data: {
        entries: Array<{
            items: {
                nested: Array<{
                    values: [number, number];
                    deep: {
                        deeper: {
                            deepest: Array<[string, number]>;
                        };
                    };
                }>;
            };
        }>;
    };
}

function brokenProcessor(config: BrokenConfig): void {
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

    const pipeline = [
        {
            name: "stage1",
            fn: (x: number) => {
                return ({
                    result: [
                        (x * 2),
                        (x * 3,
                        {
                            nested: {
                                values: [x, x + 1, x + 2],
                                extra: [
                                    { a: [1, 2, 3] },
                                    { b: [4, 5, 6] },
                                ]
                            }
                        })
                    ]
                });
            }
        },
        {
            name: "stage2",
            fn: (x: number) => {
                const deep = {
                    level1: {
                        level2: {
                            level3: [
                                [x, [x + 1, [x + 2, [x + 3, [x + 4]]]]],
                                [x * 2, [x * 3, [x * 4]]],
                            ]
                        }
                    }
                };  // <-- missing } for outer "stage2" object
            }
        }
    ];
}
