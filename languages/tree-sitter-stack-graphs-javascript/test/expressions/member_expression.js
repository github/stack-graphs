let x = 1;

// Flow in

/**/ x.foo;
//   ^ defined: 1

// Flow out

(y = 1).foo;

/**/ y;
//   ^ defined: 10

// Flow around

/**/ x;
//   ^ defined: 1

// Optional chain
/**/ x?.foo
//   ^ defined: 1