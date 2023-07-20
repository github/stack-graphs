function* () {
    let x = 1;

    // Flow in

    yield x;
    //    ^ defined: 2

    // Flow out

    yield y = 1;

    /**/ y;
    //   ^ defined: 11

    // Flow around

    /**/ x;
    //   ^ defined: 2
}