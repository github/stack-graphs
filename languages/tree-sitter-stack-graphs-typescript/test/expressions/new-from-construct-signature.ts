interface V {
    v: number;
    new () : V;
    //       ^ defined: 1
}

let x: V;
//     ^ defined: 1

new x().v;
//  ^ defined: 7
//      ^ defined: 2

export {};
