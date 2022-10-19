export {};

namespace M {
  export namespace N {
    export class A {
      f = 42;
    }
  }
}

(new M.N.A()).f;
//   ^ defined: 3
//     ^ defined: 4
//       ^ defined: 5
//            ^ defined: 6
