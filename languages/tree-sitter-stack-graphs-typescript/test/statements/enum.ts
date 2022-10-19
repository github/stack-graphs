enum E {
  C1 = 1,
  C2,
}

let h:E = true ? E.C2 : E.C1;
//    ^ defined: 1
//               ^ defined: 1
//                 ^ defined: 3
//                      ^ defined: 1
//                        ^ defined: 2

export {};
