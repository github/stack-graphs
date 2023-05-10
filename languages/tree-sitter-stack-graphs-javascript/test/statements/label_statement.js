let x = 1;

// Flow in

label: x, y = 1;
//     ^ defined: 1

// Flow out

/**/ y;
//   ^ defined: 5

// Flow around

/**/ x;
//   ^ defined: 1