type ty1 = number;

type ty2 = ty1;
//         ^ defined: 1

type ty3 = ty1 | string | ty2;
//         ^ defined: 1
//                        ^ defined: 3

type ty4 = ty1 & ty2;
//         ^ defined: 1
//               ^ defined: 3

type ty5 = ty1[];
//         ^ defined: 1

type ty6 = readonly ty1[];
//                  ^ defined: 1

type ty7 = [ty1, ty2];
//          ^ defined: 1
//               ^ defined: 3

export {};
