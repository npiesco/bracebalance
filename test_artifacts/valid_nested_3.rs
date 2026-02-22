// Deeply nested Rust — BALANCED (heavy comment & string traps)
//
// The sanitizer must ignore ALL of these:
//   // line comment: { [ ( } ] )
//   /* block comment: { [ ( } ] ) */
//   "double string: { [ ( } ] )"
//   r#"raw string: { [ ( } ] )"#
//   r##"double-hash raw: { } [ ]"##
//
// If the sanitizer fails, false positives will appear.

/*
 * Multi-line block comment with every brace type:
 *   { open curly   } close curly
 *   [ open square  ] close square
 *   ( open paren   ) close paren
 *   Nested combos: {[()]} [({})] ({[]})
 */

/// Doc comment with braces: `HashMap<String, Vec<(i32, Vec<{i32}>)>>`
/// More braces: `fn foo() { bar([1, 2, (3, 4)]) }`
use std::collections::HashMap;

/// Another doc comment: `{ [ ( unmatched in doc ) ] }`
fn main() {
    // Trap: { unclosed in line comment
    /* Trap: { unclosed in block comment */
    let _trap1 = "String trap: { [ ( } ] )";
    let _trap2 = r#"Raw trap: { unmatched [ ( "#;
    let _trap3 = r##"Double-hash trap: { } [ ] ( )"##;
    let _trap4 = "Escaped quote trap: \" still inside { [ (";
    let _trap5 = "Escaped backslash trap: \\ ending with { brace";

    /// Inner doc comment: { [ ( traps
    let config: HashMap<String, Vec<(String, HashMap<String, Vec<(i32, Vec<(i32, i32)>)>>)>> = {
        let mut root = HashMap::new();
        // comment { trap
        root.insert(
            String::from("section_a"), // { inline comment trap
            vec![
                (
                    String::from("subsection_1"), /* { block trap */
                    {
                        let mut inner = HashMap::new();
                        inner.insert(
                            String::from("key_1"),
                            vec![
                                (
                                    100,
                                    vec![
                                        (200, 300),
                                        (400, 500), // { } [ ] ( ) in comment
                                    ],
                                ),
                                (
                                    600,
                                    vec![(700, 800), (900, 1000)],
                                ),
                            ],
                        );
                        inner.insert(
                            String::from("key_2"),
                            vec![
                                (1, vec![(2, 3)]),
                                (4, vec![(5, 6), (7, 8)]),
                            ],
                        );
                        inner
                    },
                ),
                (
                    String::from("subsection_2"),
                    {
                        let mut m = HashMap::new();
                        m.insert(String::from("only"), vec![(42, vec![(1, 2)])]);
                        m
                    },
                ),
            ],
        );
        root
    };

    /*
     * The result computation below mixes closures, iterators, and
     * match expressions — all heavily nested.
     * Comment trap: {{{[[[(((
     */
    let result = {
        let data: Vec<Vec<(i32, Vec<i32>)>> = vec![
            vec![
                (
                    1,
                    vec![10, 20, 30]
                        .iter()
                        .map(|x| {
                            // inner comment trap: { [ (
                            let factor = {
                                match x {
                                    10 => {
                                        /* deep block: { [ ( */
                                        (2 + 3) * (4 + 5)
                                    }
                                    20 => {
                                        let tmp = (x * 2) + {
                                            if *x > 15 { 100 } else { 0 }
                                        };
                                        tmp
                                    }
                                    _ => {
                                        (1 + 1) * (2 + 2)
                                    }
                                }
                            };
                            *x * factor
                        })
                        .collect(),
                ),
                (
                    2,
                    vec![40, 50]
                        .iter()
                        .map(|x| {
                            (*x + 1) * {
                                if *x > 45 {
                                    (10 + {
                                        let nested = [1, 2, 3];
                                        nested.iter().sum::<i32>()
                                    })
                                } else {
                                    1
                                }
                            }
                        })
                        .collect(),
                ),
            ],
        ];
        data
    };

    // String containing code-like braces — must be sanitized
    let _code_in_string = "fn fake() { let v = vec![(1,2)]; }";
    let _raw_code = r#"fn also_fake() { match x { 1 => [2], _ => [3] } }"#;

    println!(
        "{:?}",
        (
            config.len(),
            result.len(),
        )
    );
}

/// Struct with doc comments containing braces: `Vec<(i32, {i32})>`
struct DeepStruct {
    /// Field doc: `HashMap<K, Vec<[V; N]>>` with `{ traps }`
    field: Vec<(String, HashMap<String, Vec<(i32, Vec<(i32, i32)>)>>)>,
}

/// Impl block doc: `fn new() -> Self { ... }`
impl DeepStruct {
    /// Constructor doc: returns `DeepStruct { field: vec![...] }`
    fn new() -> Self {
        DeepStruct {
            field: vec![
                (
                    String::from("entry"),
                    {
                        let mut map = HashMap::new();
                        map.insert(
                            String::from("deep_key"),
                            vec![
                                (1, vec![(2, 3), (4, 5)]),
                                (6, vec![(7, 8), (9, 10)]),
                            ],
                        );
                        map
                    },
                ),
            ],
        }
    }

    /// Method with braces in doc: `{ result.iter().map(|x| [x]) }`
    fn process(&self) -> Vec<Vec<(i32, i32)>> {
        self.field
            .iter()
            .flat_map(|(_, map)| {
                map.values().flat_map(|entries| {
                    entries.iter().map(|(_, pairs)| {
                        pairs.clone()
                    })
                })
            })
            .collect()
    }
}
