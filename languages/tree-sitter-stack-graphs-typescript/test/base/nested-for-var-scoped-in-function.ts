function f() {
  x = 2;
//^ defined: 5
  while(true) {
    for(var x = 0; x < 42; x++) {
    //             ^ defined: 5
    //                     ^ defined: 5
    }
  }
}

  x = 1; // tsc: error: Cannot find name 'x'.
//^ defined:

export {};
