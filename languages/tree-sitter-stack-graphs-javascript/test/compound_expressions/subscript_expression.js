let x = 1;

// Flow in

/**/ x[x];
//   ^ defined: 1
//     ^ defined: 1

// Flow out

(y = 1)[
    z = 1
];

/**/ y;
//   ^ defined: 11

/**/ z;
//   ^ defined: 12

// Flow around

/**/ x;
//   ^ defined: 1