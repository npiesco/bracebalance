// Deeply nested JavaScript - BALANCED

const deepConfig = {
    database: {
        connections: [
            {
                primary: {
                    host: "localhost",
                    options: {
                        pool: {
                            min: [2, 4],
                            max: [10, 20],
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
            ["L2", { size: [4096, [8192, { unit: "MB" }]] }],
        ],
    },
};

function deepProcess(input) {
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
                        }))
                        : { value: innerValue },
                }),
                {}
            ),
        }),
        {}
    );
}

module.exports = { deepConfig, deepProcess };
