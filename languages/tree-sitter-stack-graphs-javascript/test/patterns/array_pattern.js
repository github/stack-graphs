let x = 1;

// Flow In

let [y = x] = arr;
//       ^ defined: 1
// have to use assignment patterns here to get flow in

// Flow Out

let [
    z
] = arr;

/**/ z;
//   ^ defined: 12

// Flow Around

/**/ x;
//   ^ defined: 1

// Flow In From RHS

let [w = x] = x++;
//       ^ defined: 1, 25
// have to use assignment patterns here to get flow out

// Flow Into Subsequent Patterns From Earlier Patterns

let [q,
    r = q] = arr;
    //  ^ defined: 31