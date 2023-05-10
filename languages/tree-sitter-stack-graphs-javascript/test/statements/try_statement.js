let x = 1;

// Flow in

try {
    /**/ x;
    //   ^ defined: 1
    y = 1;
} catch (e) {
    /**/ e
    //   ^ defined: 9
    /**/ y;
    //   ^ defined: 8
    y = 1;
} finally {
    /**/ x;
    //   ^ defined: 1
    /**/ y;
    //   ^ defined: 8, 14
    y = 1;
}

// Flow out

/**/ y;
//   ^ defined: 8, 14, 20

// Flow around

/**/ x;
//   ^ defined: 1