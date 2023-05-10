let x = 1;

// Flow in

yield x;
//    ^ defined: 1

// Flow out

yield y = 1;

/**/ y;
//   ^ defined: 10

// Flow around

/**/ x;
//   ^ defined: 1