int opt_or_default(const int* p, int d) {
    if (p != nullptr) { return *p; } else { return d; }
}
