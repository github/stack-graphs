let x = 1;

// Flow in

if (x) {
    //   ^ defined: 1
    /**/ x;
    //   ^ defined: 1
}