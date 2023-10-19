let x = 1;
let y = (1, x);
//          ^ defined: 1

let y = (1, x = 5);
let z = x;
//      ^ defined: 1, 5
