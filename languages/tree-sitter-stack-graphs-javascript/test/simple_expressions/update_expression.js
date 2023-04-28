let x = 1;

// Flow into subexpressions

/**/ x++;
//   ^ defined: 1

// Flow around and update

/**/ x;
//   ^ defined: 1, 5