let x = 1;

// Flow in

for (let y in (x, 1)) {
    //         ^ defined: 1
    /**/ x;
    //   ^ defined: 1
    /**/ y;
    //   ^ defined: 5
    z = 1;
}

// Flow out

/**/ y;
//   ^ defined: 5

/**/ z;
//   ^ defined: 11

// Flow around

/**/ x;
//   ^ defined: 1