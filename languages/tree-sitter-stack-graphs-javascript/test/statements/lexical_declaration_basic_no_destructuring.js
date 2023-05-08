let x = 1;
/**/ x;
//   ^ defined: 1

// Flow in
let y = x;
//      ^ defined: 1

// Flow around

/**/ x;
//   ^ defined: 1

// Shadowing

let x = 2;
/**/ x;
//   ^ defined: 17