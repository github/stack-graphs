let x = 1;

// Flow in
/**/ x + x;
//   ^ defined: 1
//       ^ defined: 1

// Flow around

/**/ x;
//   ^ defined: 1

// Flow out

x + (y = 1);

/**/ y;
//   ^ defined: 15