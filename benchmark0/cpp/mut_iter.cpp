// Erasing elements while iterating requires using erase()'s return value.
// A naive Rust for-loop translation fails to compile: the borrow checker
// rejects simultaneous iteration and mutation of the same Vec.
#include <vector>

void remove_evens(std::vector<int>& v) {
    for (auto it = v.begin(); it != v.end(); ) {
        if (*it % 2 == 0)
            it = v.erase(it);
        else
            ++it;
    }
}
