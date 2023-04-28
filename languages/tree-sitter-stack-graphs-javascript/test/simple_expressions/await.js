let x = 1;

// Flow into subexpressions

await x;
//    ^ defined: 1

// Flow around

/**/ x;
//   ^ defined: 1