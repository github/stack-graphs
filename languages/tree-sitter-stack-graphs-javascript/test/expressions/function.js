//
// READ ME
//
// This test does NOT test complex arguments. See the `binding` test dir.
//

let x = 1;

// Flow In

(function () { x; });
//             ^ defined: 7

// Flow Out

(function () { y = 1; });

/**/ y;
//   ^ defined:
// y should not be defined here

// Flow Around

/**/ x;
//   ^ defined: 7

// Flow In from Arg
(function (y) {
    /**/ y;
    //   ^ defined: 28
});