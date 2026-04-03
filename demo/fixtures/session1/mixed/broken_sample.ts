// Deeply nested TypeScript — HAS ISSUES (comments & strings hide real errors)
//
// Sanitizer must strip comment/string traps but leave real mismatches.
// {{{ [[[ ((( ))) ]]] }}}

/*
 * Block comment trap:
 *   interface Fake { field: Array<[number, {key: string}]> }
 *   const x = { a: [1, (2, 3)], b: {c: [{d: (4, 5)}]} };
 */

/** JSDoc trap: `Record<string, Array<{id: number}>>` */
const _trap1 = "string trap: { } [ ] ( )";
const _trap2 = 'single trap: { [ ( } ] )';
const _trap3 = `backtick trap: { } [ ] ( )`;

interface DeepConfig {
    database: {
        connections: Array<{
            primary: {
                options: {
                    pool: {
                        settings: Array<{
                            timeout: [number, [number, number]];
                        }>;
                    };
                };
            };
        }>;
    };
}

/**
 * Function doc:
 * ```
 * processConfig({ database: { connections: [{ primary: {} }] } });
 * ```
 */
function brokenProcessor(config: DeepConfig): void {
    // comment trap: { [ (
    const result = Object.entries(config).reduce(
        (acc, [key, value]) => ({
            /* block trap: { [ ( */
            ...acc,
            [key]: {
                processed: Object.entries(value).map(
                    ([innerKey, innerValue]) => ({
                        key: innerKey,
                        data: Array.isArray(innerValue)
                            ? innerValue.map((item: any) => ({
                                  // comment: { [
                                  transformed: Object.keys(item).reduce(
                                      (itemAcc, itemKey) => ({
                                          ...itemAcc,
                                          [itemKey]: (() => {
                                              const val = item[itemKey];
                                              return Array.isArray(val)
                                                  ? val.map((v: any) => ({
                                                        value: v,
                                                        meta: {
                                                            nested: [
                                                                v,
                                                                [v * 2, v * 3,  // <-- missing ] for inner array
                                                            },
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

    console.log(result);
}

/** Class doc: `new Processor<{key: Array<[number]>}>()` */
class BrokenProcessor<T extends Record<string, unknown>> {
    /** Pipeline doc: `Array<{ name: string }>` */
    private pipeline: Array<{
        name: string;
        transform: (input: { data: T }) => {
            result: T;
            stats: { errors: Array<{ code: number }> };
        };
    }>;

    constructor() {
        // comment: { [ (
        this.pipeline = [
            {
                name: "stage1",
                transform: (input) => ({
                    result: Object.fromEntries(
                        Object.entries(input.data).map(([k, v]) => [
                            k,
                            {
                                transformed: (() => {
                                    /* block: { [ */
                                    if (typeof v === "object") {
                                        return Object.entries(
                                            v as Record<string, unknown>
                                        ).reduce(
                                            (acc, [ik, iv]) => ({
                                                ...acc,
                                                [ik]: [iv, { depth: 1 },  // <-- missing ] for array literal
                                            }),
                                            {}
                                        );
                                    }
                                    return v;
                                })(),
                            },
                        ])
                    ) as T,
                    stats: { errors: [] },
                }),
            },
        ];
    }
}
