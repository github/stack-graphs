let x = 1;

// Flow into subexpressions

/**/ x + x;
//   ^ defined: 1
//       ^ defined: 1

// Flow around

/**/ x;
//   ^ defined: 1