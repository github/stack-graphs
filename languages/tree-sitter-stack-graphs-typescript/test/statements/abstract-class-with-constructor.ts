type V = { value: number; }

abstract class A {
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

class C extends A {
//              ^ defined: 3
    constructor(y:V) {
    //            ^ defined: 1
        super(y);
        //    ^ defined: 19
    }
}

let z:C = new C(null);
//    ^ defined: 17
//            ^ defined: 17

  z.f;
//^ defined: 26
//  ^ defined: 4

export {};
