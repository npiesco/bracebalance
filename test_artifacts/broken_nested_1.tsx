// Deeply nested TSX - HAS ISSUES (missing closers)

import React, { useState } from "react";

const BrokenComponent: React.FC<{
    data: {
        sections: Array<{
            items: Array<{
                nested: { values: number[] };
            }>;
        }>;
    };
}> = ({ data }) => {
    const [expanded, setExpanded] = useState<Record<string, boolean>>({});

    return (
        <div className="broken-wrapper">
            {data.sections.map((section, sIdx) => (
                <div key={sIdx} className="section">
                    {section.items.map((item, iIdx) => (
                        <div key={iIdx} className="item">
                            <div className="values">
                                {item.nested.values.map((val, vIdx) => (
                                    <span
                                        key={vIdx}
                                        onClick={() => {
                                            setExpanded((prev) => ({
                                                ...prev,
                                                [`${sIdx}-${iIdx}-${vIdx}`]: !(
                                                    prev[`${sIdx}-${iIdx}-${vIdx}`]
                                                ),
                                            }));
                                        }}
                                        className={[
                                            "value",
                                            expanded[`${sIdx}-${iIdx}-${vIdx}`]
                                                ? "expanded"
                                                : "collapsed",
                                        ].join(" ")}
                                    >
                                        {val}
                                        {expanded[`${sIdx}-${iIdx}-${vIdx}`] && (
                                            <div className="detail">
                                                <div className="computed">
                                                    {[val * 2, val * 3].map(
                                                        (c, ci) => (
                                                            <span key={ci}>
                                                                {c}
                                                            </span>
                                                        )
                                                    }  {/* <-- missing ) for .map( */}
                                                </div>
                                                <div className="meta">
                                                    {JSON.stringify({
                                                        original: val,
                                                        nested: {
                                                            deep: {
                                                                values: [
                                                                    val,
                                                                    [val * 2, val * 3],
                                                                    {
                                                                        extra: [val * 4, val * 5
                                                                    }  {/* <-- missing ] for extra array */}
                                                                ]
                                                            }
                                                        }
                                                    })}
                                                </div>
                                            </div>
                                        )}
                                    </span>
                                ))}
                            </div>
                        </div>
                    ))}
                </div>
            ))}
        </div>
    );
};

export default BrokenComponent;
