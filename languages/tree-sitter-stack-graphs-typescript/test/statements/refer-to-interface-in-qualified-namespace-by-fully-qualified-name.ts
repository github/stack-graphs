export {};

namespace M.N {
  export interface A {}
};

interface B extends M.N.A {};
//                  ^ defined: 3
//                    ^ defined: 3
//                      ^ defined: 4
