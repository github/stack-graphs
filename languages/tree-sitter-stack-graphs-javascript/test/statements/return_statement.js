function* f() {
    let x = 1;

    // Flow in

    return x;
    //     ^ defined: 2

    // Flow out

    return y = 1;

    /**/ y;
    //   ^ defined: 11

    // Flow around

    /**/ x;
    //   ^ defined: 2
}