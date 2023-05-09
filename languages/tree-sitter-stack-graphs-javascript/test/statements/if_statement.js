let x = 1;

// Flow in

// TODO NOW add flow into the condition too
if (/**/ x) {
    //   ^ defined: 1
    /**/ x;
    //   ^ defined: 1
    y = 2;
} else if (x) {
    //     ^ defined: 1
    /**/ x;
    //   ^ defined: 1
    y = 2;
} else {
    /**/ x;
    //   ^ defined: 1
    y = 2;
}

// Flow out

/**/ y;
//   ^ defined: 10, 15, 19

// Flow around

/**/ x;
//   ^ defined: 1