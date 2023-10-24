let x = 1;

// Flow in

[0, x];
//  ^ defined: 1

// Flow out

[y = 1,
    0, y];
//     ^ defined: 10

/**/ y;
//   ^ defined: 10

// Flow around

/**/ x;
//   ^ defined: 1