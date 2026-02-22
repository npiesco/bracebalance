// Deeply nested JSX - BALANCED
// Comment with braces: { [ ( } ] )
/* Block comment { unmatched [ braces ( */
const jsxTrap1 = "String with { unmatched [ brace";
const jsxTrap2 = 'Single-quoted { [ ( traps';
const jsxTrap3 = `Template with { unmatched braces }`;

import React, { useState, useMemo } from "react";

function Dashboard({ data }) {
    const [filters, setFilters] = useState({
        active: true,
        categories: ["all"],
        range: [0, 100],
    });

    const processed = useMemo(() => (
        data.sections.map((section) => ({
            ...section,
            groups: section.groups.map((group) => ({
                ...group,
                items: group.items
                    .filter((item) => (
                        filters.active
                            ? (item.score >= filters.range[0] &&
                               item.score <= filters.range[1])
                            : true
                    ))
                    .map((item) => ({
                        ...item,
                        computed: {
                            rank: Math.floor(
                                (item.score / 100) * (
                                    group.items.length > 0
                                        ? group.items.reduce(
                                            (sum, i) => sum + i.score, 0
                                        ) / group.items.length
                                        : 1
                                )
                            ),
                            tags: [
                                ...item.tags,
                                ...(item.score > 90 ? ["top-performer"] : []),
                                ...(item.score < 10 ? ["needs-attention"] : []),
                            ],
                        },
                    })),
            })),
        }))
    ), [data, filters]);

    return (
        <div className="dashboard">
            <div className="filters">
                <div className="filter-group">
                    {["all", "active", "inactive"].map((cat) => (
                        <button
                            key={cat}
                            className={[
                                "filter-btn",
                                filters.categories.includes(cat)
                                    ? "selected"
                                    : "unselected",
                            ].join(" ")}
                            onClick={() =>
                                setFilters((prev) => ({
                                    ...prev,
                                    categories: prev.categories.includes(cat)
                                        ? prev.categories.filter((c) => c !== cat)
                                        : [...prev.categories, cat],
                                }))
                            }
                        >
                            {cat}
                        </button>
                    ))}
                </div>
            </div>
            <div className="content">
                {processed.map((section) => (
                    <div key={section.id} className="section">
                        <h2>{section.title}</h2>
                        {section.groups.map((group) => (
                            <div key={group.id} className="group">
                                <h3>{group.name}</h3>
                                <div className="items">
                                    {group.items.map((item) => (
                                        <div key={item.id} className="item-card">
                                            <div className="header">
                                                <span>{item.name}</span>
                                                <span className="score">
                                                    {item.score}
                                                </span>
                                            </div>
                                            <div className="details">
                                                <div className="rank">
                                                    Rank: {item.computed.rank}
                                                </div>
                                                <div className="tags">
                                                    {item.computed.tags.map(
                                                        (tag, idx) => (
                                                            <span
                                                                key={idx}
                                                                className="tag"
                                                            >
                                                                {tag}
                                                            </span>
                                                        )
                                                    )}
                                                </div>
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>
                ))}
            </div>
        </div>
    );
}

export default Dashboard;
