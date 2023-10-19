let x = 1;

// Flow In

let { z: y = x } = obj;
//           ^ defined: 1
// have to use assignment patterns here to get flow in

// Flow Out

let {
    z: w
} = obj;

/**/ w;
//   ^ defined: 12

// Flow Around

/**/ x;
//   ^ defined: 1

// Flow In From RHS

let { z: q = r } =
    //       ^ defined: 27
    r++;

// Flow Into Subsequent Patterns From Earlier Patterns

let { z: s,
    z: t = s } = obj;
//         ^ defined: 31

// Flow Out From Shorthand Property Identifier Pattern

let { u } = obj;

/**/ u;
//   ^ defined: 37

// Flow Out From Object Assignment Pattern

let { v = 1 } = obj;

/**/ v;
//   ^ defined: 44