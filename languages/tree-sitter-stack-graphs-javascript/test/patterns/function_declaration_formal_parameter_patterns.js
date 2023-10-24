let x = 1;

// Flow In and Out To Body

function foo(y = x, [z] = arr) {
    //           ^ defined: 1
    /**/ y;
    //   ^ defined: 5, 1
    /**/ z;
    //   ^ defined: 5
}

// Flow Out

/**/ z;
//   ^ defined: