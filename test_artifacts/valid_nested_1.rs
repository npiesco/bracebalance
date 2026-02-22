// Deeply nested Rust - BALANCED

use std::collections::HashMap;

fn main() {
    let data: HashMap<&str, Vec<(i32, Vec<HashMap<&str, Vec<(i32, i32)>>>)>> = {
        let mut outer = HashMap::new();
        outer.insert("root", vec![
            (
                1,
                vec![
                    {
                        let mut inner = HashMap::new();
                        inner.insert("branch_a", vec![
                            (10, 20),
                            (30, 40),
                        ]);
                        inner.insert("branch_b", vec![
                            (50, 60),
                            (70, 80),
                        ]);
                        inner
                    },
                    {
                        let mut inner2 = HashMap::new();
                        inner2.insert("branch_c", vec![
                            (90, 100),
                            (110, 120),
                        ]);
                        inner2
                    },
                ]
            ),
            (
                2,
                vec![
                    {
                        let mut deep = HashMap::new();
                        deep.insert("nested", vec![
                            (200, 300),
                        ]);
                        deep
                    },
                ]
            ),
        ]);
        outer
    };

    let result = {
        let processed: Vec<Vec<(i32, Vec<i32>)>> = vec![
            vec![
                (
                    1,
                    vec![2, 3, 4].iter().map(|x| {
                        let y = {
                            let z = (x * 2) + {
                                if *x > 2 { 10 } else { 0 }
                            };
                            z * (z + 1)
                        };
                        y
                    }).collect()
                ),
                (
                    5,
                    vec![6, 7, 8].iter().map(|x| {
                        (x + 1) * {
                            match x {
                                6 => { (2 + 3) * (4 + 5) },
                                7 => { (3 + 4) * (5 + 6) },
                                _ => { (1 + 1) },
                            }
                        }
                    }).collect()
                ),
            ],
        ];
        processed
    };

    println!("{:?}", (data, result));
}

struct DeepStruct {
    field: Vec<(String, HashMap<String, Vec<(i32, Vec<(i32, i32)>)>>)>,
}

impl DeepStruct {
    fn new() -> Self {
        DeepStruct {
            field: vec![
                (
                    String::from("entry"),
                    {
                        let mut map = HashMap::new();
                        map.insert(String::from("key"), vec![
                            (
                                1,
                                vec![(2, 3), (4, 5)]
                            ),
                            (
                                6,
                                vec![(7, 8), (9, 10)]
                            ),
                        ]);
                        map
                    }
                ),
            ],
        }
    }
}
