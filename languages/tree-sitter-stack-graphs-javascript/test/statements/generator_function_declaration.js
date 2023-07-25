//
// READ ME
//
// This test does NOT test complex arguments. See the `binding` test dir.
//

let x = 1;

// Flow In

function* foo() {
    /**/ x;
    //   ^ defined: 7
}

// Flow Out

function* bar() {
    y = 1;
}

/**/ y;
//   ^ defined:
// y should not be defined here

// Flow Around

/**/ x;
//   ^ defined: 7

// Flow In from Arg
function* baz(y) {
    /**/ y;
    //   ^ defined: 32
}