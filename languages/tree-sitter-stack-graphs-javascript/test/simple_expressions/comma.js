let x = 1;

// Flow in

(1, x);
//  ^ defined: 1

(y = 1, y);
//      ^ defined: 8

// Flow out

(1, z = 5);

/**/ y;
//   ^ defined: 8
/**/ z;
//   ^ defined: 13

// Flow around

/**/ x;
//   ^ defined: 1