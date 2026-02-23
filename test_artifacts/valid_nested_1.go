package main

// Go valid nested brackets
// Tests: backtick raw strings, line comments, nested maps/slices

import "fmt"

// Backtick raw string — braces inside are ignored
var schema = `
    type Query {
        users: [User]
        user(id: ID): User
    }
`

func processNested(input map[string][]int) map[string]int {
	result := make(map[string]int)
	for key, vals := range input {
		sum := 0
		for _, v := range vals {
			sum += v
		}
		result[key] = sum
	}
	return result
}

func buildPipeline(stages []func(int) int) func(int) int {
	return func(x int) int {
		for _, stage := range stages {
			x = stage(x)
		}
		return x
	}
}

func main() {
	data := map[string][]int{
		"a": {1, 2, 3},
		"b": {4, 5, 6},
	}

	nested := map[string]map[string][]int{
		"x": {"p": {1, 2}, "q": {3, 4}},
		"y": {"r": {5, 6}},
	}

	_ = processNested(data)

	pipeline := buildPipeline([]func(int) int{
		func(n int) int { return n * 2 },
		func(n int) int { return n + 1 },
	})

	for k, inner := range nested {
		for ik, iv := range inner {
			fmt.Printf("%s.%s: %v -> %d\n", k, ik, iv, pipeline(iv[0]))
		}
	}
}
