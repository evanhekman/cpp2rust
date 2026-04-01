int new_delete_ptr() {
    int* p = new int(5);
    int x = *p;
    delete p;
    return x;
}

