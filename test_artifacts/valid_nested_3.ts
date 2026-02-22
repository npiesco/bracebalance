// Deeply nested TypeScript — BALANCED (heavy comment & string traps)
//
// Sanitizer must strip ALL of these:
//   // line: { [ ( } ] )
//   /* block: { [ ( } ] ) */
//   "double": { [ ( } ] )
//   'single': { [ ( } ] )
//   `backtick`: { [ ( } ] )
//
// {{{ [[[ ((( ))) ]]] }}}

/*
 * Multi-line block with deep brace nesting:
 *   interface Fake { field: Array<[number, {key: string}]> }
 *   const x = { a: [1, (2, 3)], b: {c: [{d: (4, 5)}]} };
 */

/** JSDoc with braces: `Record<string, Array<{id: number}>>` */
const _trap1 = "string trap: { } [ ] ( ) {[()]}";
const _trap2 = 'single trap: { [ ( } ] )';
const _trap3 = `backtick trap: { unmatched } [ ] ( )`;
const _trap4 = "escaped: \" still in string { [ (";
const _trap5 = `escaped backtick: \` still { in template`;
const _trap6 = "backslash at end: \\ with { brace";

/**
 * Interface doc: `{ connections: Array<{host: string}> }`
 * More: `Map<string, Set<[number, {value: any}]>>`
 */
interface DeepConfig {
    database: {
        connections: Array<{
            primary: {
                host: string;
                options: {
                    pool: {
                        min: [number, number];
                        max: [number, number];
                        callbacks: Array<{
                            name: string;
                            params: {
                                retries: [number, number, number];
                                meta: { tags: Array<[string, { priority: number }]> };
                            };
                        }>;
                    };
                };
            };
        }>;
    };
}

/**
 * Function doc with code block:
 * ```
 * processConfig({
 *     database: { connections: [{ primary: { host: "x" } }] }
 * });
 * ```
 */
function processConfig(config: DeepConfig): void {
    // comment trap: { [ (
    const result = Object.entries(config).reduce(
        (acc, [key, value]) => ({
            /* block trap: { [ */
            ...acc,
            [key]: {
                processed: Object.entries(value).map(
                    ([innerKey, innerValue]) => ({
                        key: innerKey,
                        data: Array.isArray(innerValue)
                            ? innerValue.map((item: any) => ({
                                  // inner comment: { [
                                  transformed: Object.keys(item).reduce(
                                      (itemAcc, itemKey) => ({
                                          ...itemAcc,
                                          [itemKey]: (() => {
                                              /* deep block: { ( */
                                              const val = item[itemKey];
                                              return Array.isArray(val)
                                                  ? val.map((v: any) => ({
                                                        value: v,
                                                        meta: {
                                                            type: typeof v,
                                                            nested: [
                                                                v,
                                                                [v * 2, v * 3],
                                                            ],
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

/** Class doc: `new Processor<{key: Array<[number]>}>()` */
class DeepProcessor<T extends Record<string, unknown>> {
    /**
     * Pipeline doc: `Array<{ name: string; transform: (x: T) => T }>`
     * Braces: { [ ( ) ] }
     */
    private pipeline: Array<{
        name: string;
        transform: (input: { data: T; meta: { depth: number } }) => {
            result: T;
            stats: {
                processed: number;
                errors: Array<{ code: number; msg: string }>;
            };
        };
    }>;

    constructor() {
        // constructor comment: { [ (
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
                                    if (
                                        typeof v === "object" &&
                                        v !== null
                                    ) {
                                        /* block: { [ ( */
                                        return Object.entries(
                                            v as Record<string, unknown>
                                        ).reduce(
                                            (acc, [ik, iv]) => ({
                                                ...acc,
                                                [ik]: [
                                                    iv,
                                                    {
                                                        depth:
                                                            input.meta.depth +
                                                            1,
                                                    },
                                                ],
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
