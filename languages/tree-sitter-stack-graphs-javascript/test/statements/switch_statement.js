let x = 1;

// Flow in

switch (x) {
    //  ^ defined: 1
    case value:
        /**/ x;
        //   ^ defined: 1
        y = 2;
    default:
        /**/ x;
        //   ^ defined: 1
        y = 2;
}

// Flow out

/**/ y;
//   ^ defined: 10, 16

// Flow around

/**/ x;
//   ^ defined: 1