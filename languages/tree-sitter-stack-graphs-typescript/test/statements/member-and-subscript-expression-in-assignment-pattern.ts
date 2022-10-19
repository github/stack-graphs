let x = {
    y: 0
};

  [ x.y, x["y"] ] = [ 42, 42 ];
//  ^ defined: 1
//    ^ defined: 2
//       ^ defined: 1
//         ^ defined: 2
