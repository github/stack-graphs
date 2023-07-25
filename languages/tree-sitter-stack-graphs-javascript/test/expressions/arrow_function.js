//
// READ ME
//
// This test does NOT test complex arguments. See the `binding` test dir.
//

let x = 1;

// Flow In

() => x;
//    ^ defined: 7

() => { x };
//      ^ defined: 7

// Flow Out

() => y = 1;

/**/ y;
//   ^ defined:
// y should not be defined here

// Flow Around

/**/ x;
//   ^ defined: 7

// Flow In from Arg
y =>
    y + 1;
//  ^ defined: 31