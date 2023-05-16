let x = 1;

// Flow in

{
    /**/[x]: x
    //   ^ defined: 1
    //       ^ defined: 1
};

// Flow out

{
    [y = 0]:
    /**/ (y, z = 0)
    //    ^ defined: 14
};

/**/ y;
//   ^ defined: 14

/**/ z;
//   ^ defined: 15