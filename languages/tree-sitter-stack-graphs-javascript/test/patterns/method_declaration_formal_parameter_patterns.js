let x = 1;

// Flow In and Out To Body

class Foo {
    foo(y = x, [z] = arr) {
        //  ^ defined: 1
        /**/ y;
        //   ^ defined: 6, 1
        /**/ z;
        //   ^ defined: 6
    }
}

// Flow Out

/**/ z;
//   ^ defined: