let x = 1;

// Flow In

[y = x] = 2;
//   ^ defined: 1
// have to use array patterns or something else here to get a pattern
// on the LHS that can contain an assignment pattern

// Flow Out

[z = 1] = 2;
/**/ z;
//   ^ defined: 12
// have to use array patterns or something else here to get a pattern
// on the LHS that can contain an assignment pattern

// Flow Around

/**/ x;
//   ^ defined: 1

// Flow In From RHS

[w = x] = x++;
//   ^ defined: 1, 25
// have to use array patterns or something else here to get a pattern
// on the LHS that can contain an assignment pattern