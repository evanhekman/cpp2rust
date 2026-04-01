/*
increment two references
- the c++ implementation allows both reference parameters to alias the same variable
- calling add_both(x, x) is legal in c++, so both increments apply to the same integer
- a naive rust translation using fn add_both(a: &mut i32, b: &mut i32) would be rejected because rust forbids two simultaneous mutable borrows of the same value
- a good rust translation changes the api to reflect the aliasing case explicitly, such as taking one &mut i32 and incrementing it twice, or using Cell<i32> if shared aliasing is required
*/

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