let x = 1;

// Flow in

`template ${x} string`;
//          ^ defined: 1

`template ${y = 1} ${y} string`;
//                   ^ defined: 8


// Flow out

/**/ y;
//   ^ defined: 8

// Flow around

/**/ x;
//   ^ defined: 1