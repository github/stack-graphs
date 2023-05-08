let x = 1;

// Flow in
yield x;
//    ^ defined: 1

// Flow around

/**/ x;
//   ^ defined: 1