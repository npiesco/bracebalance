// Deeply nested TSX — BALANCED (heavy comment & string traps)
//
// Sanitizer must strip:
//   // line: { [ ( } ] )
//   /* block: { [ ( } ] ) */
//   "double": { [ ( } ] )
//   'single': { [ ( } ] )
//   `template`: { [ ( } ] )

/*
 * Block:
 *   interface Fake { items: Array<[string, {key: number}]> }
 *   <Component prop={{ key: [1, (2, 3)] }} />
 */

/** @returns {React.ReactElement} with `{ deeply: [nested, (types)] }` */
import React, { useState, useCallback, useMemo } from "react";

const _trap1 = "TSX string: { } [ ] ( ) {[()]}";
const _trap2 = 'single trap: { [ ( } ] )';
const _trap3 = `template: { unmatched } [ ] ( )`;
const _trap4 = "escaped: \" still { inside string";

/**
 * Interface doc: `{ layers: Array<{items: [{nested: {deep: number[]}}]}> }`
 */
interface DeepConfig {
    layers: Array<{
        items: Array<{
            nested: { deep: { values: number[] } };
        }>;
    }>;
}

/** Component doc with braces: `Record<string, {visible: boolean}>` */
const DeepComponent: React.FC<{
    config: DeepConfig;
}> = ({ config }) => {
    // comment: { [ (
    const [state, setState] = useState<{
        expanded: Record<
            string,
            { children: Array<{ id: string; visible: boolean }> }
        >;
    }>({ expanded: {} });

    /* block: { [ ( */
    const processed = useMemo(
        () =>
            config.layers.map((layer, layerIdx) => ({
                key: layerIdx,
                items: layer.items.map((item, itemIdx) => ({
                    key: `${layerIdx}-${itemIdx}`,
                    // inner comment: { } [ ] ( )
                    values: item.nested.deep.values.reduce(
                        (acc, val) => ({
                            ...acc,
                            [`v${val}`]: {
                                original: val,
                                computed: [val * 2, val * 3, val * 4],
                            },
                        }),
                        {} as Record<
                            string,
                            { original: number; computed: number[] }
                        >
                    ),
                })),
            })),
        [config]
    );

    /**
     * Handler doc:
     * ```
     * handleClick({ id: "x", data: { nested: { values: [1, 2] } } });
     * ```
     */
    const handleClick = useCallback(
        (params: {
            id: string;
            data: { nested: { values: [number, number] } };
        }) => {
            /* block: { [ */
            setState((prev) => ({
                expanded: {
                    ...prev.expanded,
                    [params.id]: {
                        children: Object.entries(prev.expanded).map(
                            ([key, val]) => ({
                                id: key,
                                visible: val.children.some(
                                    (child) => child.id === params.id
                                ),
                            })
                        ),
                    },
                },
            }));
        },
        []
    );

    return (
        <div className="deep-wrapper">
            {/* JSX comment: { [ ( } ] ) */}
            {processed.map((layer) => (
                <div key={layer.key} className="layer">
                    {layer.items.map((item) => (
                        <div key={item.key} className="item">
                            {Object.entries(item.values).map(
                                ([valKey, valData]) => (
                                    <div key={valKey} className="row">
                                        <span>{valData.original}</span>
                                        <div className="computed">
                                            {valData.computed.map(
                                                (c, i) => (
                                                    <span
                                                        key={i}
                                                        onClick={() =>
                                                            handleClick({
                                                                id: valKey,
                                                                data: {
                                                                    nested: {
                                                                        values:
                                                                            [
                                                                                c,
                                                                                i,
                                                                            ],
                                                                    },
                                                                },
                                                            })
                                                        }
                                                    >
                                                        {c}
                                                    </span>
                                                )
                                            )}
                                        </div>
                                    </div>
                                )
                            )}
                        </div>
                    ))}
                </div>
            ))}
        </div>
    );
};

export default DeepComponent;
