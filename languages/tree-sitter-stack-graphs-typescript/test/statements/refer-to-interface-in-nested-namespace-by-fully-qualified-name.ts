export {};

namespace M {
  export namespace N {
    export interface A {}
  };
};

interface B extends M.N.A {};
//                  ^ defined: 3
//                    ^ defined: 4
//                      ^ defined: 5
