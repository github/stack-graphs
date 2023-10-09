let x = 1;

// Flow in

for (let y = (x, 1);
    //        ^ defined: 1
    /**************/ x < y;
    //               ^ defined: 1
    //                   ^ defined: 5, 10
    /*********************/ y = (x, y)) {
    //                           ^ defined: 1
    //                              ^ defined: 5, 10
    /**/ x;
    //   ^ defined: 1
    /**/ y;
    //   ^ defined: 5, 10
    z = 1;
}

// Flow out

/**/ y;
//   ^ defined: 5, 10

/**/ z;
//   ^ defined: 17

// Flow around

/**/ x;
//   ^ defined: 1