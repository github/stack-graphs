let x = 1;
let f = 2;

// Flow in
/**/ f(x);
//     ^ defined: 1
//   ^ defined: 2

// Flow around

/**/ x;
//   ^ defined: 1

// Flow out

(y = 1)(
    z = 2
);

/**/ y;
//   ^ defined: 16

/**/ z;
//   ^ defined: 17