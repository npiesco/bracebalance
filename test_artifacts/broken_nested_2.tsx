// Deeply nested TSX — HAS ISSUES (comments & strings hide real errors)
//
// Sanitizer must strip traps but leave real mismatches.

/*
 * Block trap:
 *   interface Fake { items: Array<[string, {key: number}]> }
 *   <Component prop={{ key: [1, (2, 3)] }} />
 */

/** Component doc: `{ layers: Array<{items: [{nested: {deep: number[]}}]}> }` */
import React, { useState, useCallback, useMemo } from "react";

const _trap1 = "TSX string trap: { } [ ] ( )";
const _trap2 = 'single trap: { [ ( } ] )';
const _trap3 = `template trap: { } [ ] ( )`;

interface BrokenConfig {
    layers: Array<{
        items: Array<{
            nested: { deep: { values: number[] } };
        }>;
    }>;
}

/**
 * Component doc:
 * ```
 * <BrokenComponent config={{ layers: [{ items: [{ nested: { deep: { values: [1] } } }] }] }} />
 * ```
 */
const BrokenComponent: React.FC<{
    config: BrokenConfig;
}> = ({ config }) => {
    // comment: { [ (
    const [state, setState] = useState<{
        expanded: Record<
            string,
            { children: Array<{ id: string; visible: boolean }> }
        >;
    }>({ expanded: {} });

    /* block trap: { [ ( */
    const processed = useMemo(
        () =>
            config.layers.map((layer, layerIdx) => ({
                key: layerIdx,
                items: layer.items.map((item, itemIdx) => ({
                    key: `${layerIdx}-${itemIdx}`,
                    // comment: { } [ ]
                    values: item.nested.deep.values.reduce(
                        (acc, val) => ({
                            ...acc,
                            [`v${val}`]: {
                                original: val,
                                computed: [val * 2, val * 3,  // <-- missing ] for computed array
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
     * Handler doc with braces: `setState({ expanded: { [id]: { children: [] } } })`
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
                <div key={layer.key}>
                    {layer.items.map((item) => (
                        <div key={item.key}>
                            {Object.entries(item.values).map(
                                ([valKey, valData]) => (
                                    <div key={valKey}>
                                        <span>{valData.original}</span>
                                        <div>
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
                                            }
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

export default BrokenComponent;
