// Deeply nested JavaScript - BALANCED
// Comment traps: { [ ( } ] )
/* Block { [ ( */
/*
 * Extended block:
 *   function fake() { return [1, {2: (3)}]; }
 *   const obj = { a: [1, (2, 3)], b: {c: [{d: 4}]} };
 */
/** @returns {{ result: Array<[number, {key: string}]> }} */
const jsTrap = "{ string [ brace ( trap";
const jsTrap2 = `template { literal [ trap`;
const jsTrap3 = 'single: { [ ( } ] )';
const jsTrap4 = "escaped: \" still { in string";
const jsTrap5 = `escaped: \` still { in template`;

class EventSystem {
    constructor() {
        this.handlers = new Map([
            [
                "click",
                {
                    listeners: [
                        {
                            id: 1,
                            callback: (event) => ({
                                handled: true,
                                result: {
                                    position: [event.x, event.y],
                                    meta: {
                                        timestamp: Date.now(),
                                        modifiers: {
                                            keys: [
                                                event.ctrlKey ? ["ctrl"] : [],
                                                event.shiftKey ? ["shift"] : [],
                                            ].flat(),
                                            extra: {
                                                computed: [
                                                    [1, [2, [3, [4, [5]]]]],
                                                    [6, [7, [8]]],
                                                ],
                                            },
                                        },
                                    },
                                },
                            }),
                            options: {
                                once: false,
                                capture: true,
                                filters: [
                                    (e) => (e.target.matches(".btn") && (e.type === "click")),
                                    (e) => (!e.defaultPrevented),
                                ],
                            },
                        },
                    ],
                },
            ],
        ]);
    }

    process(events) {
        return events.reduce(
            (results, event) => ({
                ...results,
                [event.type]: {
                    count: (results[event.type]?.count || 0) + 1,
                    outcomes: [
                        ...(results[event.type]?.outcomes || []),
                        ...Array.from(this.handlers.get(event.type)?.listeners || [])
                            .filter((listener) =>
                                listener.options.filters.every((filter) => filter(event))
                            )
                            .map((listener) => ({
                                listenerId: listener.id,
                                result: listener.callback(event),
                                timing: {
                                    queued: Date.now(),
                                    processed: Date.now() + Math.floor(
                                        Math.random() * (100 - 10 + 1) + 10
                                    ),
                                },
                            })),
                    ],
                },
            }),
            {}
        );
    }
}

const pipeline = [
    {
        stage: "transform",
        run: (data) => {
            return data.map((item) => ({
                ...item,
                transformed: {
                    values: item.values.map((v) => ({
                        original: v,
                        derived: [
                            v * 2,
                            {
                                squared: v * v,
                                cubed: v * v * v,
                                factors: Array.from(
                                    { length: v },
                                    (_, i) => (i + 1)
                                ).filter((f) => (v % f === 0)),
                            },
                        ],
                    })),
                },
            }));
        },
    },
];

module.exports = { EventSystem, pipeline };
