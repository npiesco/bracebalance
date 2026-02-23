package main

// Go broken nested brackets
// Backtick raw string contains decoy braces — sanitizer must strip them

var template = `
    if (condition) {
        action([payload]);
    }
`

func brokenProcess(data map[string][]int) map[string]int {
	result := map[string]int{
		"init": 0,
	}

	for key, vals := range data {
		for _, v := range vals {
			if v > 0 {
				result[key] += v
			// <-- missing } for inner if block
		}
		// <-- missing } for inner for loop

	// <-- missing } for outer for loop

	return result
// <-- missing } for brokenProcess function
