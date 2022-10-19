type V = { value: number; }

class C {
    set p(v: V) {}
    //       ^ defined: 1
}

let x:C;

  x.p = null;
//^ defined: 8
//  ^ defined: 4

  x['p'] = null;
//^ defined: 8
//  ^ defined: 4

export {};
