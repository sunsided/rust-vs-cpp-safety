// Example 1, C++ side: compiles cleanly, but push_back reallocates and
// `first` dangles. Build with -fsanitize=address to see it.
#include <iostream>
#include <vector>

int main() {
    std::vector<int> values = {10, 20, 30};
    int& first = values[0];
    values.push_back(40);
    std::cout << first << '\n';
}
