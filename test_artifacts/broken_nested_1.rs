// Deeply nested Rust - HAS ISSUES (missing closers)

use std::collections::HashMap;

fn broken_main() {
    let data = {
        let mut map = HashMap::new();
        map.insert("key", vec![
            (
                1,
                vec![
                    {
                        let mut inner = HashMap::new();
                        inner.insert("nested", vec![
                            (10, 20),
                            (30, 40),
                        ]);
                        inner
                    },
                    {
                        let mut inner2 = HashMap::new();
                        inner2.insert("deep", vec![
                            (50, vec![
                                (60, 70),
                                (80, 90),
                            ),  // <-- missing ] for vec![ on line 23
                        ]);
                        inner2
                    },
                ]
            ),
        ]);
        map
    };

    let result = vec![
        (
            "stage1",
            vec![1, 2, 3].iter().map(|x| {
                let y = {
                    let z = (x * 2) + {
                        if *x > 2 { 10 } else { 0 }
                    };
                    z * (z + 1)
                };
                y
            }).collect::<Vec<_>>()
        ),
        (
            "stage2",
            vec![4, 5, 6].iter().map(|x| {
                (*x + 1) * {
                    match x {
                        4 => { (2 + 3) * (4 + 5) },
                        5 => { (3 + 4) * (5 + 6) },
                        _ => { (1 + 1 },  // <-- missing ) for inner expression
                    }
                }
            }).collect::<Vec<_>>()
        ),
    ];

    let complex = {
        let items: Vec<(i32, Vec<(i32, i32)>)> = vec![
            (
                1,
                vec![(2, 3), (4, 5]  // <-- missing ) for tuple
            ),
            (
                6,
                vec![(7, 8), (9, 10)]
            ),
        ];
        items
    };
}
