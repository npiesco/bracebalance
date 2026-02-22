// Deeply nested TSX - BALANCED
// Comment traps: { [ ( } ] )
/* Block { [ ( */
const tsxTrap = "{ string [ brace ( trap";
const tsxTrap2 = `template { literal [ trap`;

import React, { useReducer, createContext, useContext } from "react";

type State = {
    entities: Record<string, {
        data: Array<{
            id: number;
            nested: {
                layers: Array<{
                    items: [string, { value: number; tags: string[] }][];
                }>;
            };
        }>;
    }>;
};

type Action =
    | { type: "ADD"; payload: { key: string; entry: State["entities"][string]["data"][0] } }
    | { type: "REMOVE"; payload: { key: string; id: number } };

const reducer = (state: State, action: Action): State => {
    switch (action.type) {
        case "ADD":
            return {
                entities: {
                    ...state.entities,
                    [action.payload.key]: {
                        data: [
                            ...(state.entities[action.payload.key]?.data ?? []),
                            action.payload.entry,
                        ],
                    },
                },
            };
        default:
            return state;
    }
};

const DeepContext = createContext<{
    state: State;
    dispatch: React.Dispatch<Action>;
}>({ state: { entities: {} }, dispatch: () => {} });

const DeepProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [state, dispatch] = useReducer(reducer, { entities: {} });
    return (
        <DeepContext.Provider value={{ state, dispatch }}>
            {children}
        </DeepContext.Provider>
    );
};

const EntityDisplay: React.FC<{
    entityKey: string;
}> = ({ entityKey }) => {
    const { state } = useContext(DeepContext);
    const entity = state.entities[entityKey];

    if (!entity) return (<div className="empty">No data</div>);

    return (
        <div className="entity">
            <h2>{entityKey}</h2>
            {entity.data.map((entry) => (
                <div key={entry.id} className="entry">
                    <span className="id">{entry.id}</span>
                    {entry.nested.layers.map((layer, layerIdx) => (
                        <div key={layerIdx} className="layer">
                            {layer.items.map(([label, detail], itemIdx) => (
                                <div key={itemIdx} className="item">
                                    <div className="label">{label}</div>
                                    <div className="value">{detail.value}</div>
                                    <div className="tags">
                                        {detail.tags.map((tag, tagIdx) => (
                                            <span
                                                key={tagIdx}
                                                className={[
                                                    "tag",
                                                    tag.startsWith("!")
                                                        ? "important"
                                                        : "normal",
                                                ].join(" ")}
                                            >
                                                {tag}
                                            </span>
                                        ))}
                                    </div>
                                </div>
                            ))}
                        </div>
                    ))}
                </div>
            ))}
        </div>
    );
};

const App: React.FC = () => {
    return (
        <DeepProvider>
            <div className="app">
                <EntityDisplay entityKey="users" />
                <EntityDisplay entityKey="products" />
            </div>
        </DeepProvider>
    );
};

export default App;
