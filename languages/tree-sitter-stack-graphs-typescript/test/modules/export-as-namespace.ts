/*--- path: ModA.ts ---*/

export let x = {
  v: 42
};

export as namespace A;

/*--- path: ModB.ts ---*/

  A.x.v;
//^ defined: 7
//  ^ defined: 3
//    ^ defined: 4

export {};
