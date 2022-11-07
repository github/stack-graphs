enum E {
  C
}

let y = E;
//      ^ defined: 1

  y.C;
//^ defined: 5
//  ^ defined: 2

export {};
