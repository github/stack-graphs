let x = 1;

// Flow in

[0, x];
//  ^ defined: 1

// Flow out

[y = 1,
    0, y];
//     ^ defined: 10

/**/ y;
//   ^ defined: 10

// Flow around

/**/ x;
//   ^ defined: 1

let arr = [0, x];
let x2 = arg[0];

/**/ x2;
//   ^ defined: 23
// let one = arr[1].x;
// //        defined: 1
// //               defined: 2
