let x = 1;

// Flow in

do {
    /**/ x;
    //   ^ defined: 1
    /**/ y;
    //   ^ defined: 11
    z = 1;
} while ((x, y = 5));
//        ^ defined: 1

// Flow out

/**/ y;
//   ^ defined: 11

/**/ z;
//   ^ defined: 10

// Flow around

/**/ x;
//   ^ defined: 1
