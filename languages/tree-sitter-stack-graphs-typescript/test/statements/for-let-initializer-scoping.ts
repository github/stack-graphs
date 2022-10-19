for(let x = 1; x < 42; x++) {
//             ^ defined: 1
//                     ^ defined: 1
  x;
//^ defined: 1
}

  x; // tsc: Cannot find name 'x'.
//^ defined:

export {};
