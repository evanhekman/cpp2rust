int nullable_ptr(int* p) {
    if (p == nullptr) {
        return -1;
    }
    if (!p) {
        return -2;
    }
    return *p;
}

