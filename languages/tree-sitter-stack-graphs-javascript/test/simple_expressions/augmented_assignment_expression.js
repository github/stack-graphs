let x = 1;
let y = 2;
let z = 3;

// Flow into subexpressions

z += x;
//   ^ defined: 1

// Flow around and update

y += 1;

/**/ y;
//   ^ defined: 2, 12