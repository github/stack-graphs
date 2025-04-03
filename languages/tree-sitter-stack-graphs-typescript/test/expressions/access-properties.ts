{
    let x = { y: 42 };
    x.y;
//    ^ defined: 2
}

{ /// Deep
    let x = { y: { z: 42 } };
    x.y.z;
//      ^ defined: 8
}

{ /// Deep with parenthesized expression
    let x = { y: { z: 42 } };
    (x).y.z;
//        ^ defined: 14
}

{ /// Deep with subscript expression
    let x = [{ y: { z: 42 } }];
    x[0].y.z;
//         ^ defined: 20
}
