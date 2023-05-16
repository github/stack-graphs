let x = 1;
let y = 2;

let z = foo(x, y++, y);
//          ^ defined: 1, 4
//             ^ defined: 2, 4, 4
//                  ^ defined: 2, 4, 4
