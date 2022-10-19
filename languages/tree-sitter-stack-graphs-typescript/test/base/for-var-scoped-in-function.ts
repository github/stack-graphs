function f() {
  x = 2;
//^ defined: 4
  for(var x = 0; x < 42; x++) {
  //             ^ defined: 4
  //                     ^ defined: 4
  }
}

  x = 1; // tsc: error: Cannot find name 'x'.
//^ defined:

export {};
