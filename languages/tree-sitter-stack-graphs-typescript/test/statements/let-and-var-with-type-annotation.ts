type my_number = number;

let x1:my_number = 42;
//     ^ defined: 1

let x2:my_number;
//     ^ defined: 1

var x3:my_number = 42;
//     ^ defined: 1

var x4:my_number;
//     ^ defined: 1

   x1 + x2 + x3 + x4;
// ^ defined: 3
     // ^ defined: 6
          // ^ defined: 9
               // ^ defined: 12

export {};
