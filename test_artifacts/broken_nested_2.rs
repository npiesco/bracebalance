// Deeply nested Rust — HAS ISSUES (comments & strings hide real errors)
//
// This file has genuine balance errors buried among comment/string traps.
// The sanitizer must strip the traps but leave the real mismatches.

/*
 * Block comment trap: { [ ( } ] )
 * These are fine — they're in a comment.
 */

/// Doc comment trap: `Vec<(i32, {key})>` — also fine
use std::collections::HashMap;

fn broken_with_comments() {
    // comment: { [ ( — ignored
    let _trap = "string with { [ ( braces }";
    let _raw = r#"raw string { [ ( }"#;

    let config: HashMap<String, Vec<(i32, Vec<(i32, i32)>)>> = {
        let mut map = HashMap::new();
        /* block comment trap: { { { */
        map.insert(
            String::from("section"),
            vec![
                (
                    1,
                    vec![
                        (10, 20),
                        (30, 40),
                    ],
                ),
                (
                    2,
                    vec![
                        (50, 60),
                        (70, 80  // <-- missing ) for tuple
                    ],
                ),
            ],
        );
        map
    };

    // comment: perfectly fine { [ (
    let result = {
        let data: Vec<Vec<(i32, Vec<i32>)>> = vec![
            vec![
                (
                    1,
                    vec![10, 20, 30]
                        .iter()
                        .map(|x| {
                            /* block trap: { [ */
                            let factor = {
                                match x {
                                    10 => {
                                        // comment { trap
                                        (2 + 3) * (4 + 5)
                                    }
                                    _ => {
                                        (1 + 1 * (2 + 2)  // <-- missing ) around 1 + 1
                                    }
                                }
                            };
                            *x * factor
                        })
                        .collect(),
                ),
            ],
        ];
        data
    };

    /// Doc comment: `fn fake() { vec![(1,2)] }`
    let nested = {
        let items: Vec<(String, HashMap<String, Vec<(i32, i32)>>)> = vec![
            (
                String::from("entry"),
                {
                    let mut m = HashMap::new();
                    m.insert(
                        String::from("key"),
                        vec![
                            (1, 2),
                            (3, 4,  // <-- missing ) for tuple
                        ],
                    );
                    m
                },
            ),
        ];
        items
    };
}
