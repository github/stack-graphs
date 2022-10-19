enum E {
  C
}

var x: E;
//     ^ defined: 1

  x.C; // tsc: Property 'C' does not exist on type 'E'.
//^ defined: 5
//  ^ defined:

export {};
