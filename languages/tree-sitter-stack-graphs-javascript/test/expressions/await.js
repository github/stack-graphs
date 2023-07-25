let x = 1;

// Flow in
await x;
//    ^ defined: 1

// Flow around

/**/ x;
//   ^ defined: 1

// Flow out

await y = 1;

/**/ y;
//   ^ defined: 14