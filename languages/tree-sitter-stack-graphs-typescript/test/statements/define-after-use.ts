var x: T;
//     ^ defined: 15

{
    var i: U;
    //     ^ defined: 8

    type U = T;
    //       ^ defined: 15

    var j: U;
    //     ^ defined: 8
}

type T = number;

var y: T;
//     ^ defined: 15

var z: U; // tsc: Cannot find name 'U'
//     ^ defined:

export {};
