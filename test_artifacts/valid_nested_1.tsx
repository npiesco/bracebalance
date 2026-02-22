// Deeply nested TSX - BALANCED

import React, { useState, useCallback, useMemo } from "react";

interface TreeNode {
    id: string;
    children: Array<{
        node: TreeNode;
        meta: { depth: number; path: string[] };
    }>;
    data: {
        values: [number, number];
        nested: { items: Array<[string, { score: number }]> };
    };
}

const DeepComponent: React.FC<{
    config: {
        layers: Array<{
            items: Array<{
                nested: { deep: { values: number[] } };
            }>;
        }>;
    };
}> = ({ config }) => {
    const [state, setState] = useState<{
        expanded: Record<string, {
            children: Array<{ id: string; visible: boolean }>;
        }>;
    }>({ expanded: {} });

    const processed = useMemo(() => (
        config.layers.map((layer, layerIdx) => ({
            key: layerIdx,
            items: layer.items.map((item, itemIdx) => ({
                key: `${layerIdx}-${itemIdx}`,
                values: item.nested.deep.values.reduce(
                    (acc, val) => ({
                        ...acc,
                        [`v${val}`]: {
                            original: val,
                            computed: [val * 2, val * 3, val * 4],
                        },
                    }),
                    {} as Record<string, { original: number; computed: number[] }>
                ),
            })),
        }))
    ), [config]);

    const handleClick = useCallback(
        (params: {
            id: string;
            data: { nested: { values: [number, number] } };
        }) => {
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
            {processed.map((layer) => (
                <div key={layer.key} className="layer">
                    {layer.items.map((item) => (
                        <div key={item.key} className="item">
                            <div className="nested-content">
                                {Object.entries(item.values).map(
                                    ([valKey, valData]) => (
                                        <div key={valKey} className="value-row">
                                            <span className="original">
                                                {valData.original}
                                            </span>
                                            <div className="computed">
                                                {valData.computed.map(
                                                    (c, i) => (
                                                        <span
                                                            key={i}
                                                            className={`comp-${i}`}
                                                            onClick={() =>
                                                                handleClick({
                                                                    id: `${valKey}-${i}`,
                                                                    data: {
                                                                        nested: {
                                                                            values: [
                                                                                valData.original,
                                                                                c,
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
                        </div>
                    ))}
                </div>
            ))}
        </div>
    );
};

export default DeepComponent;
