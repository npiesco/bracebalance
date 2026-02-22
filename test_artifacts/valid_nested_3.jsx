// Deeply nested JSX — BALANCED (heavy comment & string traps)
//
// Sanitizer must strip:
//   // line: { [ ( } ] )
//   /* block: { [ ( } ] ) */
//   "double": { [ ( } ] )
//   'single': { [ ( } ] )
//   `backtick`: { [ ( } ] )

/*
 * Block comment:
 *   <Component prop={{ key: [1, (2, 3)] }} />
 *   { conditionalRender && <div>{value}</div> }
 */

/** @param {{ data: Array<{id: number, items: [string]}> }} props */
import React, { useState, useMemo, useCallback } from "react";

const _trap1 = "JSX string: { } [ ] ( ) {[()]}";
const _trap2 = 'single trap: { [ ( } ] )';
const _trap3 = `backtick: { unmatched } and [ ] and ( )`;

/**
 * Dashboard component.
 * @returns {React.ReactElement}
 * Usage: `<Dashboard data={{ sections: [{ groups: [{items: []}] }] }} />`
 */
function Dashboard({ data }) {
    // comment trap: { [ (
    const [filters, setFilters] = useState({
        active: true,
        categories: ["all"],
        range: [0, 100],
    });

    /* block trap: { [ ( */
    const processed = useMemo(
        () =>
            data.sections.map((section) => ({
                ...section,
                groups: section.groups.map((group) => ({
                    ...group,
                    // inner comment: { [
                    items: group.items
                        .filter((item) =>
                            filters.active
                                ? item.score >= filters.range[0] &&
                                  item.score <= filters.range[1]
                                : true
                        )
                        .map((item) => ({
                            ...item,
                            computed: {
                                rank: Math.floor(
                                    (item.score / 100) *
                                        (group.items.length > 0
                                            ? group.items.reduce(
                                                  (sum, i) =>
                                                      sum + i.score,
                                                  0
                                              ) / group.items.length
                                            : 1)
                                ),
                                tags: [
                                    ...item.tags,
                                    ...(item.score > 90
                                        ? ["top-performer"]
                                        : []),
                                    ...(item.score < 10
                                        ? ["needs-attention"]
                                        : []),
                                ],
                            },
                        })),
                })),
            })),
        [data, filters]
    );

    /** @type {(cat: string) => void} */
    const handleFilter = useCallback(
        (cat) => {
            /* block: { [ ( */
            setFilters((prev) => ({
                ...prev,
                categories: prev.categories.includes(cat)
                    ? prev.categories.filter((c) => c !== cat)
                    : [...prev.categories, cat],
            }));
        },
        []
    );

    return (
        <div className="dashboard">
            <div className="filters">
                {/* JSX comment: { [ ( } ] ) */}
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
                            onClick={() => handleFilter(cat)}
                        >
                            {cat}
                        </button>
                    ))}
                </div>
            </div>
            <div className="content">
                {processed.map((section) => (
                    <div key={section.id}>
                        <h2>{section.title}</h2>
                        {section.groups.map((group) => (
                            <div key={group.id}>
                                <h3>{group.name}</h3>
                                {group.items.map((item) => (
                                    <div key={item.id} className="card">
                                        <span>{item.name}</span>
                                        <span>{item.computed.rank}</span>
                                        <div>
                                            {item.computed.tags.map(
                                                (tag, i) => (
                                                    <span key={i}>
                                                        {tag}
                                                    </span>
                                                )
                                            )}
                                        </div>
                                    </div>
                                ))}
                            </div>
                        ))}
                    </div>
                ))}
            </div>
        </div>
    );
}

export default Dashboard;
