type my_number = number;

let x1:my_number = 42;
//     ^ defined: 1

let x2 = my_number;
//       ^ defined:

let x3:x2 = 42;
//     ^ defined:

export {};
