#include <iostream>

void add_both(int& a, int& b) {
    a += 1;
    b += 1;
}

int main() {
    int x = 0;
    add_both(x, x);   // legal C++
    std::cout << x;   // x becomes 2
}