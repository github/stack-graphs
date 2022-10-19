type V = { value: number; }

class C {
    f: V;
    // ^ defined: 1

    constructor(x:V) {
        //        ^ defined: 1
        this.f = { value: x.value };
        //   ^ defined: 4
        //                ^ defined: 7
        //                  ^ defined: 1
    }

}

let y:C = new C(null);
//    ^ defined: 3
//            ^ defined: 3

  y.f;
//^ defined: 17
//  ^ defined: 4

export {};
