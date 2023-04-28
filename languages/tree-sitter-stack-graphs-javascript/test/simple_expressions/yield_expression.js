let x = 1;

// Flow into subexpressions

yield x;
//    ^ defined: 1

// Flow around

/**/ x;
//   ^ defined: 1