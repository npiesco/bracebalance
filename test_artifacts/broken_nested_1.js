// Deeply nested JavaScript - HAS ISSUES (missing closers)
// Comment trap: { [ ( } ] )
/* Block comment trap:
 *   const x = { key: [1, (2, 3] };
 *   return [{ broken: (val }];
 */
/**
 * @param {{ data: Array<[string, {items: number[]}> }} input
 * @returns {{ result: Array<(number, [string])> }}
 */
const _trap = "string trap: { [ ( } ] )";
const _trap2 = `template trap: ${ "{ [ (" } ] )`;
const _trap3 = 'single trap: { [ ( } ] )';

const brokenConfig = {
    database: {
        connections: [
            {
                primary: {
                    host: "localhost",
                    options: {
                        pool: {
                            min: [2, 4],
                            max: [10, 20,  // <-- missing ] for max array
                            settings: [
                                { timeout: [30, [60, 90]] },
                                { retry: [1, [2, 3]] },
                            ],
                        },
                    },
                },
                replicas: [
                    {
                        host: "replica1",
                        weights: [
                            [1, 2, 3],
                            [4, 5, 6],
                            {
                                computed: [
                                    [Math.max(1, 2), Math.min(3, 4)],
                                    [Math.max(5, 6), Math.min(7, 8)],
                                ],
                            },
                        ],
                    },
                ],
            },
        ],
    },
    cache: {
        layers: [
            ["L1", { size: [1024, [2048, { unit: "KB" }]] }],
            ["L2", { size: [4096, [8192, { unit: "MB" }] }],  // <-- missing ] before }
        ],
    },
};

function brokenProcess(input) {
    return Object.entries(input).reduce(
        (acc, [key, value]) => ({
            ...acc,
            [key]: Object.entries(value).reduce(
                (innerAcc, [innerKey, innerValue]) => ({
                    ...innerAcc,
                    [innerKey]: Array.isArray(innerValue)
                        ? innerValue.map((item) => ({
                            processed: Object.entries(item).reduce(
                                (itemAcc, [itemKey, itemVal]) => ({
                                    ...itemAcc,
                                    [itemKey]: (() => {
                                        if (Array.isArray(itemVal)) {
                                            return itemVal.map((v) => ({
                                                value: v,
                                                meta: {
                                                    nested: {
                                                        deep: {
                                                            values: [
                                                                v,
                                                                [v * 2, v * 3],
                                                                { extra: [v * 4] },
                                                            ],
                                                        },
                                                    },
                                                },
                                            }));
                                        }
                                        return { raw: itemVal };
                                    })(),
                                }),
                                {}
                            ),
                        })  // <-- missing ) for .map(
                        : { value: innerValue },
                }),
                {}
            ),
        }),
        {}
    );
}
