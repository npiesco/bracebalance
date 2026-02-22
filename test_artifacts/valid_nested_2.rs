// Deeply nested Rust - BALANCED
// Brace traps: { [ ( } ] )
/* Block comment { unmatched [ ( */
/*
 * Extended block trap:
 *   fn fake() { vec![(1, {2: [3]})] }
 *   match x { Foo(y) => [y], Bar{z} => (z) }
 */
/// Doc comment trap: `HashMap<String, Vec<(i32, {key})>>` 
/// More doc: `fn foo() -> Result<(), Box<dyn Error>> { Ok(()) }`
const TRAP: &str = "{ unmatched [ brace in string";
const RAW_TRAP: &str = r#"{ raw [ string ( trap"#;
const RAW2: &str = r##"double hash raw: { [( )]} "##;
const ESCAPED: &str = "escaped \" quote with { brace";
const BACKSLASH: &str = "trailing backslash \\ then { brace";

enum DeepEnum {
    Variant1(Vec<(i32, Box<DeepEnum>)>),
    Variant2 {
        data: HashMap<String, Vec<(String, Vec<i32>)>>,
        nested: Option<Box<DeepEnum>>,
    },
    Leaf(i32),
}

use std::collections::HashMap;

fn build_deep() -> DeepEnum {
    DeepEnum::Variant2 {
        data: {
            let mut map = HashMap::new();
            map.insert(
                String::from("layer1"),
                vec![
                    (
                        String::from("a"),
                        vec![1, 2, 3],
                    ),
                    (
                        String::from("b"),
                        (0..10).filter(|x| {
                            (x % 2 == 0) && {
                                let check = (x * x) + (x + 1);
                                check < 50
                            }
                        }).collect(),
                    ),
                ],
            );
            map
        },
        nested: Some(Box::new(
            DeepEnum::Variant1(
                vec![
                    (
                        1,
                        Box::new(DeepEnum::Variant1(
                            vec![
                                (
                                    2,
                                    Box::new(DeepEnum::Variant1(
                                        vec![
                                            (
                                                3,
                                                Box::new(DeepEnum::Leaf(42))
                                            ),
                                        ]
                                    ))
                                ),
                            ]
                        ))
                    ),
                    (
                        10,
                        Box::new(DeepEnum::Variant2 {
                            data: {
                                let mut inner = HashMap::new();
                                inner.insert(
                                    String::from("deep"),
                                    vec![(String::from("z"), vec![99, 100])],
                                );
                                inner
                            },
                            nested: None,
                        })
                    ),
                ]
            )
        )),
    }
}

fn process_deep(val: &DeepEnum) -> Vec<i32> {
    match val {
        DeepEnum::Leaf(n) => vec![*n],
        DeepEnum::Variant1(entries) => {
            entries.iter().flat_map(|(n, child)| {
                let mut result = vec![*n];
                result.extend(
                    process_deep(child).iter().map(|x| {
                        (*x) + ({
                            let bonus = if *n > 5 { 100 } else { 0 };
                            bonus
                        })
                    })
                );
                result
            }).collect()
        },
        DeepEnum::Variant2 { data, nested } => {
            let mut result: Vec<i32> = data
                .values()
                .flat_map(|entries| {
                    entries.iter().flat_map(|(_, nums)| {
                        nums.iter().map(|n| {
                            (*n) * (if *n > 5 { 2 } else { 1 })
                        })
                    })
                })
                .collect();
            if let Some(child) = nested {
                result.extend(process_deep(child));
            }
            result
        },
    }
}
