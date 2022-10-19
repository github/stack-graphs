type V = { value: number; }

class C {
    get p(): V { return null; }
    //       ^ defined: 1
}

let x:C;

  x.p.value;
//^ defined: 8
//  ^ defined: 4
//    ^ defined: 1

  x['p'].value;
//^ defined: 8
//  ^ defined: 4
//       ^ defined: 1

export {};
