let x = 1;

// Flow in

switch (x) {
    //  ^ defined: 1
    case 0:
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
//   ^ defined: 10, 14

// Flow around

/**/ x;
//   ^ defined: 1