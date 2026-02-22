// Deeply nested JSX - BALANCED
// Comment traps: { [ ( } ] )
/* Block { [ ( */
const jsxTrap = "{ string [ brace ( trap";
const jsxTrap2 = `template { literal [ trap`;

import React, { useReducer, useCallback } from "react";

const initialState = {
    tree: {
        root: {
            children: [
                {
                    id: "a",
                    data: { values: [1, [2, [3, [4]]]] },
                    children: [
                        {
                            id: "a1",
                            data: { values: [5, [6, [7]]] },
                            children: [],
                        },
                    ],
                },
            ],
        },
    },
};

function reducer(state, action) {
    switch (action.type) {
        case "TOGGLE":
            return {
                ...state,
                tree: {
                    ...state.tree,
                    root: {
                        ...state.tree.root,
                        children: state.tree.root.children.map((child) => ({
                            ...child,
                            expanded: child.id === action.id
                                ? !child.expanded
                                : child.expanded,
                            children: child.children.map((grandchild) => ({
                                ...grandchild,
                                highlighted: (
                                    grandchild.data.values.some((v) =>
                                        Array.isArray(v)
                                            ? v.some((inner) =>
                                                Array.isArray(inner)
                                                    ? inner.includes(action.target)
                                                    : inner === action.target
                                            )
                                            : v === action.target
                                    )
                                ),
                            })),
                        })),
                    },
                },
            };
        default:
            return state;
    }
}

function TreeNode({ node, depth, onToggle }) {
    const indent = { paddingLeft: `${depth * 20}px` };

    return (
        <div style={indent} className="tree-node">
            <div
                className={[
                    "node-header",
                    node.expanded ? "expanded" : "collapsed",
                    node.highlighted ? "highlighted" : "",
                ].filter(Boolean).join(" ")}
                onClick={() => onToggle({
                    id: node.id,
                    target: node.data.values[0],
                })}
            >
                <span className="toggle">
                    {node.children.length > 0 ? (
                        node.expanded ? "[-]" : "[+]"
                    ) : "[ ]"}
                </span>
                <span className="label">{node.id}</span>
                <span className="values">
                    ({node.data.values.map((v) => (
                        Array.isArray(v) ? `[${v}]` : String(v)
                    )).join(", ")})
                </span>
            </div>
            {node.expanded && node.children.length > 0 && (
                <div className="children">
                    {node.children.map((child) => (
                        <TreeNode
                            key={child.id}
                            node={child}
                            depth={depth + 1}
                            onToggle={onToggle}
                        />
                    ))}
                </div>
            )}
        </div>
    );
}

function TreeView() {
    const [state, dispatch] = useReducer(reducer, initialState);

    const handleToggle = useCallback(
        ({ id, target }) => {
            dispatch({ type: "TOGGLE", id, target });
        },
        []
    );

    return (
        <div className="tree-view">
            <h1>Deep Tree</h1>
            {state.tree.root.children.map((child) => (
                <TreeNode
                    key={child.id}
                    node={child}
                    depth={0}
                    onToggle={handleToggle}
                />
            ))}
        </div>
    );
}

export default TreeView;
