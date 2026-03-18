int control_flow(int x) {
    int y = 0;

    if (x < 0) {
        y = -x;
    } else if (x == 0) {
        y = 10;
    } else {
        y = x;
    }

    for (int i = 0; i < 3; i++) {
        y = y + i;
    }

    int j = 0;
    while (j < 2) {
        j++;
        if (j == 1) continue;
        break;
    }

    switch (x) {
        case 1: y = 111; break;
        default: y = 222; break;
    }

    return y;
}

