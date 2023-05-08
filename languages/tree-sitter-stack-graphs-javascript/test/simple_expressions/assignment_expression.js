let x = 1;
let y = 2;
let z = 3;

// Flow in
z = x;
//  ^ defined: 1

// Flow around and update

x = (y = 3);

/**/ x;
//   ^ defined: 1, 12

/****/ y;
//     ^ defined: 2, 12