var x = 1;
/**/ x;
//   ^ defined: 1

// Flow in
var y = x;
//      ^ defined: 1

// Flow around

/**/ x;
//   ^ defined: 1

// Shadowing

var x = 2;
/**/ x;
//   ^ defined: 16