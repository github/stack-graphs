let x = 1;

// Flow In
throw x, y = 1;
//    ^ defined: 1

// Flow Out

/**/ y;
//   ^ defined: 4

// Flow around

/**/ x;
//   ^ defined: 1
