let x = 1;
const y = 2;

var z = x + y;
//      ^ defined: 1
//          ^ defined: 2

let w = z;
//      ^ defined: 1, 2, 4
