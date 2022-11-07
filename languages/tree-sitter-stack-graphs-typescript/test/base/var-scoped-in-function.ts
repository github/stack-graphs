function f() {
  x = 2;
//^ defined: 4
  var x;
}

  x = 1; // tsc: error: Cannot find name 'x'.
//^ defined:


export {};
