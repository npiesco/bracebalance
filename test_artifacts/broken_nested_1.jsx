// Deeply nested JSX - HAS ISSUES (missing closers)

import React, { useState } from "react";

function BrokenList({ items }) {
    const [selected, setSelected] = useState(null);

    return (
        <div className="list-wrapper">
            {items.map((section, sIdx) => (
                <div key={sIdx} className="section">
                    <h2>{section.title}</h2>
                    {section.groups.map((group, gIdx) => (
                        <div key={gIdx} className="group">
                            <h3>{group.name}</h3>
                            {group.entries.map((entry, eIdx) => (
                                <div
                                    key={eIdx}
                                    className={[
                                        "entry",
                                        selected === entry.id ? "selected" : "",
                                    ].join(" ")}
                                    onClick={() => {
                                        setSelected(entry.id);
                                        console.log({
                                            action: "select",
                                            data: {
                                                id: entry.id,
                                                nested: {
                                                    values: [
                                                        entry.score,
                                                        [entry.score * 2, entry.score * 3],
                                                        {
                                                            computed: [
                                                                (entry.score + 1),
                                                                (entry.score + 2,  // <-- missing ) for tuple
                                                            ]
                                                        }
                                                    ]
                                                }
                                            }
                                        });
                                    }}
                                >
                                    <div className="entry-content">
                                        <span>{entry.name}</span>
                                        <div className="badges">
                                            {entry.tags.map((tag, tIdx) => (
                                                <span
                                                    key={tIdx}
                                                    className={[
                                                        "badge",
                                                        tag.important
                                                            ? "important"
                                                            : "normal",
                                                    ].join(" ")}
                                                >
                                                    {tag.label}
                                                </span>
                                            )}
                                        </div>
                                        {entry.details && (
                                            <div className="details">
                                                {Object.entries(entry.details).map(
                                                    ([dk, dv], dIdx) => (
                                                        <div key={dIdx}>
                                                            <strong>{dk}:</strong>
                                                            <span>
                                                                {JSON.stringify(dv)}
                                                            </span>
                                                        </div>
                                                    )
                                                }  {/* <-- missing ) for .map( */}
                                            </div>
                                        )}
                                    </div>
                                </div>
                            ))}
                        </div>
                    ))}
                </div>
            )}  {/* <-- missing ) for .map( on line 11 */}
        </div>
    );
}

export default BrokenList;
