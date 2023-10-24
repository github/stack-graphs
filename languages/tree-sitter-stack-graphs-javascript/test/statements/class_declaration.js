let x = 1;

// Flow In

class Foo extends x {
    //            ^ defined: 1
    z = x;
    //  ^ defined: 1

    bar() {
        /**/ x;
        //   ^ defined: 1

        /**/ z;
        //   ^ defined:
        // z should not be defined here
    }
}

// Flow Out

class Baz {
    y = 1;
}

/**/ y;
//   ^ defined:
// y should not be defined here

// Flow Around

/**/ x;
//   ^ defined: 1