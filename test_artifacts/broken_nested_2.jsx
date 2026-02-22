// Deeply nested JSX — HAS ISSUES (comments & strings hide real errors)
//
// Sanitizer must strip traps but leave real mismatches.

/*
 * Block trap:
 *   <Component prop={{ key: [1, (2, 3)] }} />
 *   { conditionalRender && <div>{value}</div> }
 */

/** @param {{ data: Array<{id: number}> }} props */
import React, { useState, useMemo } from "react";

const _trap1 = "JSX string trap: { } [ ] ( )";
const _trap2 = 'single trap: { [ ( } ] )';
const _trap3 = `backtick trap: { } [ ] ( )`;

/**
 * @returns {React.ReactElement}
 * Usage: `<Dashboard data={{ sections: [{ groups: [{items: []}] }] }} />`
 */
function BrokenDashboard({ data }) {
    // comment: { [ (
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
                    items: group.items
                        .filter((item) =>
                            filters.active
                                ? item.score >= filters.range[0] &&
                                  item.score <= filters.range[1]
                                : true
                        )
                        .map((item) => ({
                            ...item,
                            // comment: { [
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
                                        : [],  // <-- this is fine
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

    return (
        <div className="dashboard">
            {/* JSX comment: { [ ( } ] ) */}
            <div className="content">
                {processed.map((section) => (
                    <div key={section.id}>
                        {section.groups.map((group) => (
                            <div key={group.id}>
                                {group.items.map((item) => (
                                    <div key={item.id}>
                                        <span>{item.name}</span>
                                        <div>
                                            {item.computed.tags.map(
                                                (tag, i) => (
                                                    <span key={i}>
                                                        {tag}
                                                    </span>
                                                )
                                            }
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

export default BrokenDashboard;
