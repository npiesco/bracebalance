// Deeply nested JavaScript — BALANCED (heavy comment & string traps)
//
// Sanitizer must ignore:
//   // line: { [ ( } ] )
//   /* block: { [ ( } ] ) */
//   "double": { [ ( } ] )
//   'single': { [ ( } ] )
//   `backtick`: { [ ( } ] )
//
// {{{ [[[ ((( ))) ]]] }}}

/*
 * Multi-line block:
 *   const fake = { a: [1, (2, 3)], b: {c: [{d: (4, 5)}]} };
 *   function fake2() { return [1, {2: (3)}]; }
 */

/** @param {Object} config - The { deeply } nested [config] (object) */
const _trap1 = "string: { } [ ] ( ) {[()]}";
const _trap2 = 'single: { [ ( } ] )';
const _trap3 = `backtick: { unmatched } [ ] ( )`;
const _trap4 = "escaped: \" still { in string";
const _trap5 = `escaped: \` still { in template`;

/**
 * @returns {{ result: Array<[number, {key: string}]> }}
 * More JSDoc braces: { [ ( ) ] }
 */
function buildConfig() {
    // comment: { unclosed [
    const config = {
        database: {
            /* block: { [ ( */
            connections: [
                {
                    primary: {
                        host: "localhost",
                        options: {
                            pool: {
                                min: [2, 4],
                                max: [10, 20],
                                settings: [
                                    {
                                        timeout: [30, [60, 90]],
                                        // comment: { } [ ] ( )
                                        retry: {
                                            attempts: [1, 2, 3],
                                            backoff: {
                                                params: {
                                                    base: [100, 200],
                                                    jitter: {
                                                        range: [0.1, 0.5],
                                                    },
                                                },
                                            },
                                        },
                                    },
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
                                        [
                                            Math.max(1, 2),
                                            Math.min(3, 4),
                                        ],
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
                [
                    "L1",
                    { size: [1024, [2048, { unit: "KB" }]] },
                ],
                [
                    "L2",
                    { size: [4096, [8192, { unit: "MB" }]] },
                ],
            ],
        },
    };

    return config;
}

/**
 * Deep processor.
 * @param {{ data: Object, meta: { depth: number } }} input
 * @returns {{ result: Object, stats: { count: number } }}
 */
function deepProcess(input) {
    /* block trap: { [ ( */
    return Object.entries(input).reduce(
        (acc, [key, value]) => ({
            ...acc,
            [key]: Object.entries(value).reduce(
                (innerAcc, [innerKey, innerValue]) => ({
                    ...innerAcc,
                    // comment: { [ (
                    [innerKey]: Array.isArray(innerValue)
                        ? innerValue.map((item) => ({
                              processed: Object.entries(item).reduce(
                                  (itemAcc, [itemKey, itemVal]) => ({
                                      ...itemAcc,
                                      [itemKey]: (() => {
                                          // inner: { [
                                          if (Array.isArray(itemVal)) {
                                              return itemVal.map((v) => ({
                                                  value: v,
                                                  meta: {
                                                      nested: {
                                                          deep: {
                                                              values: [
                                                                  v,
                                                                  [v * 2],
                                                                  {
                                                                      extra: [
                                                                          v * 3,
                                                                      ],
                                                                  },
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

module.exports = { buildConfig, deepProcess };
