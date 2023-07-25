let x = 1;

// Flow in
/**/ (x);
//    ^ defined: 1

// Flow out

(y = 1);

/**/ y;
//   ^ defined: 9

// Flow around

/**/ x;
//   ^ defined: 1