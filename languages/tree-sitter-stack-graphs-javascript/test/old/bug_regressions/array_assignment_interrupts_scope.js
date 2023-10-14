let bar = {};

/**/ bar;
//   ^ defined: 1

/**/ bar["one"] = 1;
//   ^ defined: 1

/**/ bar;
//   ^ defined: 1
