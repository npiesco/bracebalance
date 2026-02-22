// Deeply nested JavaScript — HAS ISSUES (comments & strings hide real errors)
//
// Sanitizer must strip traps but leave real mismatches.
// {{{ [[[ ((( ))) ]]] }}}

/*
 * Block comment trap:
 *   const fake = { a: [1, (2, 3)], b: {c: [{d: (4, 5)}]} };
 */

/** @param {Object} config - The { deeply } nested [config] (object) */
const _trap1 = "string trap: { } [ ] ( )";
const _trap2 = 'single trap: { [ ( } ] )';
const _trap3 = `backtick trap: { } [ ] ( )`;

/**
 * @returns {{ result: Array<[number, {key: string}]> }}
 */
function brokenProcess(input) {
    // comment trap: { [ (
    return Object.entries(input).reduce(
        (acc, [key, value]) => ({
            /* block trap: { [ ( */
            ...acc,
            [key]: Object.entries(value).reduce(
                (innerAcc, [innerKey, innerValue]) => ({
                    ...innerAcc,
                    [innerKey]: Array.isArray(innerValue)
                        ? innerValue.map((item) => ({
                              // comment: { [
                              processed: Object.entries(item).reduce(
                                  (itemAcc, [itemKey, itemVal]) => ({
                                      ...itemAcc,
                                      [itemKey]: (() => {
                                          if (Array.isArray(itemVal)) {
                                              /* deep block: { */
                                              return itemVal.map((v) => ({
                                                  value: v,
                                                  meta: {
                                                      nested: {
                                                          deep: {
                                                              values: [
                                                                  v,
                                                                  [v * 2,  // <-- missing ] for inner array
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

/**
 * Deep config builder.
 * @returns {{ database: { connections: Array<Object> } }}
 */
function brokenConfig() {
    // comment: { [ (
    return {
        database: {
            /* block: { [ */
            connections: [
                {
                    primary: {
                        options: {
                            pool: {
                                min: [2, 4],
                                max: [10, 20],
                                settings: [
                                    {
                                        timeout: [30, [60, 90]],
                                        retry: {
                                            attempts: [1, 2, 3,  // <-- missing ] for attempts array
                                            backoff: {
                                                params: {
                                                    base: [100, 200],
                                                },
                                            },
                                        },
                                    },
                                ],
                            },
                        },
                    },
                },
            ],
        },
    };
}

module.exports = { brokenProcess, brokenConfig };
