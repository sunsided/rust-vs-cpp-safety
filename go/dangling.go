// Example 1, Go side: memory safe, but `first` silently stops aliasing
// `values`. append reallocates; the GC keeps the old array alive, so the
// write through `first` lands in a copy nobody else sees.
package main

import "fmt"

func main() {
	values := []int{10, 20, 30}
	first := &values[0]

	values = append(values, 40)

	*first = 99

	fmt.Println(values[0]) // 10
	fmt.Println(*first)    // 99
}
